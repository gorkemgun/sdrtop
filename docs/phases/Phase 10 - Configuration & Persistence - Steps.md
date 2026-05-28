# Phase 10 — Configuration & Persistence: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)

**Goal:** Radio settings (frequency, gains, amp) and display state (active preset,
waterfall row count) survive application restarts. Settings are read from
`~/.config/sdrtop/config.toml` at startup and written back on clean exit.
CLI flags (`--frequency`, `--lna`, `--vga`) override config file values.

**Prerequisite:** Phase 9 complete. `serde` and `toml` are already in `Cargo.toml`.
Only `clap` is new.

---

## Correctness notes

**Parse errors must never crash the app.** If the config file exists but cannot
be parsed, `load_or_default` logs a warning to stderr and returns `Default::default()`.
The user can then fix their config; the app still starts.

**Atomic save.** Write to `config.toml.tmp`, then `rename` to `config.toml`.
`rename` is atomic on Linux (same filesystem). A crash mid-write leaves a `.tmp`
file but leaves the previous `config.toml` intact.

**CLI flags override config.** The merge order is:
`Default::default()` → config file → CLI args.
This means `--frequency 433000000` wins over whatever is in the file.

**Hardware application at startup.** After `Device::open()`, all saved radio
settings are applied to the hardware with best-effort (errors pushed to the in-app
log, startup continues). The hardware must be configured before RX streaming
begins — but `rx_enabled` starts `false`, so the polling task won't call
`start_rx` until Space is pressed.

**`save_config()` must not hold the mutex during file I/O.** Extract all
needed values from `SdrMetrics` while the lock is held, drop the guard, then
write the file. File writes are not instantaneous; blocking on disk while holding
the metrics mutex would delay the 100 ms render loop.

**`serde(default)` on nested structs.** A config file with only `[radio]` and
no `[display]` section must not fail to parse — missing sections get
`Default::default()`. Annotate both fields on `AppConfig` with `#[serde(default)]`
and derive `Default` for `RadioConfig` and `DisplayConfig`.

---

## Dependency order

```
Cargo.toml          add clap 4
    ↓
src/config.rs       RadioConfig + DisplayConfig + AppConfig
                    + load_or_default() + save()
                    + 4 unit tests
    ↓
src/main.rs         Cli struct (clap) + default_config_path()
                    + restructured main(): parse → load → override → App::new
    ↓
src/app.rs          App::new(cfg, config_path)
                    apply settings to hardware + SdrMetrics
                    save_config() helper
                    'q' key calls save_config() before returning
```

---

## Step 1 — `clap` dependency + `AppConfig` struct + tests

**Files:** `Cargo.toml`, `src/config.rs`

- [ ] **Add to `Cargo.toml` `[dependencies]`:**

```toml
clap = { version = "4", features = ["derive"] }
```

`serde` and `toml` are already present — no change needed there.

- [ ] **Add to `src/config.rs`** — insert at the top of the file, after the
  existing `use` lines:

```rust
use serde::Serialize;
use std::path::Path;

use crate::state::{DEFAULT_FREQUENCY, DEFAULT_LNA_GAIN, DEFAULT_SAMPLE_RATE, DEFAULT_VGA_GAIN};
```

`Deserialize` is already imported via the existing `use serde::Deserialize;` line.
If that line is absent, change it to `use serde::{Deserialize, Serialize};`.

- [ ] **Add the three config structs** — insert before the existing `pub enum Position`:

```rust
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RadioConfig {
    pub frequency_hz: u64,
    pub sample_rate: f64,
    pub lna_gain: u32,
    pub vga_gain: u32,
    pub amp_enabled: bool,
}

impl Default for RadioConfig {
    fn default() -> Self {
        Self {
            frequency_hz: DEFAULT_FREQUENCY,
            sample_rate:  DEFAULT_SAMPLE_RATE,
            lna_gain:     DEFAULT_LNA_GAIN,
            vga_gain:     DEFAULT_VGA_GAIN,
            amp_enabled:  false,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DisplayConfig {
    pub active_preset: String,
    pub waterfall_max_rows: usize,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            active_preset:      "minimal".into(),
            waterfall_max_rows: 64,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub radio: RadioConfig,
    #[serde(default)]
    pub display: DisplayConfig,
}
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Add 4 unit tests** to the existing `mod tests` block in `src/config.rs`:

```rust
    #[test]
    fn default_radio_config_frequency() {
        assert_eq!(RadioConfig::default().frequency_hz, 2_400_000_000);
        assert_eq!(RadioConfig::default().lna_gain, 16);
    }

    #[test]
    fn load_or_default_missing_file_returns_default() {
        let cfg = AppConfig::load_or_default(Path::new("/nonexistent/sdrtop/config.toml"));
        assert_eq!(cfg.radio.frequency_hz, RadioConfig::default().frequency_hz);
    }

    #[test]
    fn deserialize_partial_toml_fills_missing_with_defaults() {
        let toml = "[radio]\nfrequency_hz = 433_000_000\n";
        let cfg: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.radio.frequency_hz, 433_000_000);
        assert_eq!(cfg.display.active_preset, "minimal");
    }

    #[test]
    fn serialize_deserialize_round_trip() {
        let mut cfg = AppConfig::default();
        cfg.radio.lna_gain = 24;
        cfg.display.active_preset = "spectrum".into();
        let serialized = toml::to_string_pretty(&cfg).unwrap();
        let restored: AppConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(restored.radio.lna_gain, 24);
        assert_eq!(restored.display.active_preset, "spectrum");
    }
```

These tests reference `AppConfig::load_or_default` which doesn't exist yet —
the `load_missing_file` test will fail to compile until Step 2. Add the tests
now; they will compile after Step 2.

- [ ] **Run `cargo build`**. Expected compile error on the two tests that reference
  `load_or_default`. This is expected — fixed in Step 2.

---

## Step 2 — `load_or_default()` + `save()`

**Files:** `src/config.rs`

- [ ] **Add `impl AppConfig`** — insert after the `AppConfig` struct definition:

```rust
impl AppConfig {
    pub fn load_or_default(path: &Path) -> Self {
        let content = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => return Self::default(),
        };
        match toml::from_str(&content) {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("Warning: failed to parse {}: {e}. Using defaults.", path.display());
                Self::default()
            }
        }
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, content)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }
}
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Run `cargo test config::tests`**. Expected: all 7 tests pass (3 existing + 4 new).

---

## Step 3 — CLI args + `default_config_path()` + updated `main()`

**Files:** `src/main.rs`

Replace the entire contents of `src/main.rs` with:

```rust
mod app;
mod config;
mod dsp;
mod event;
mod fft;
mod hardware;
mod palette;
mod state;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use config::AppConfig;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "sdrtop", about = "HackRF One / PortaPack terminal monitor")]
struct Cli {
    /// Path to config file (default: ~/.config/sdrtop/config.toml)
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Center frequency in Hz, e.g. 433920000 (overrides config)
    #[arg(long, value_name = "HZ")]
    frequency: Option<u64>,

    /// LNA gain in dB, 0–40 step 8 (overrides config)
    #[arg(long)]
    lna: Option<u32>,

    /// VGA gain in dB, 0–62 step 2 (overrides config)
    #[arg(long)]
    vga: Option<u32>,
}

fn default_config_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".config/sdrtop/config.toml"))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_path = cli.config.or_else(default_config_path);
    let mut app_cfg = config_path
        .as_deref()
        .map(AppConfig::load_or_default)
        .unwrap_or_default();

    // CLI args override config file values
    if let Some(f) = cli.frequency { app_cfg.radio.frequency_hz = f; }
    if let Some(l) = cli.lna       { app_cfg.radio.lna_gain = l.min(40); }
    if let Some(v) = cli.vga       { app_cfg.radio.vga_gain = v.min(62); }

    let mut app = match App::new(app_cfg, config_path) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Application error: {:?}", err);
    }

    Ok(())
}
```

- [ ] **Run `cargo build`**. Expected compile errors on `App::new` (wrong arity) —
  fixed in Step 4.

---

## Step 4 — `App::new()` signature + apply settings + `save_config()` + 'q' key

**Files:** `src/app.rs`

- [ ] **Update imports** at the top of `src/app.rs`:

```rust
use std::path::PathBuf;

use crate::config::{AppConfig, DisplayConfig, LayoutConfig, RadioConfig};
```

Replace the existing `use crate::config::LayoutConfig;` line with the above.

- [ ] **Add `config_path` field to `App` struct**:

```rust
pub struct App {
    state: Arc<Mutex<SdrMetrics>>,
    #[allow(dead_code)]
    device: Arc<hardware::Device>,
    #[allow(dead_code)]
    rx_ctx: Arc<RxContext>,
    config_path: Option<PathBuf>,
    events: EventStream,
    show_help: bool,
    engine: ui::LayoutEngine,
}
```

- [ ] **Change `App::new()` signature**:

```rust
pub fn new(cfg: AppConfig, config_path: Option<PathBuf>) -> anyhow::Result<Self> {
```

- [ ] **Apply saved radio settings to hardware** — add after the board info is
  read (after `let serial = device.serial_number()?;`) and before `let state = ...`:

```rust
        // Apply saved radio settings to hardware before streaming starts
        let startup_results = [
            device.set_frequency(cfg.radio.frequency_hz),
            device.set_sample_rate(cfg.radio.sample_rate),
            device.set_lna_gain(cfg.radio.lna_gain),
            device.set_vga_gain(cfg.radio.vga_gain),
            device.set_amp_enable(cfg.radio.amp_enabled),
        ];
```

(The results are checked after `state` is created so errors can be logged.)

- [ ] **Update `SdrMetrics { ... }` initialization** — replace the hardcoded
  constant fields with values from `cfg.radio`:

```rust
        let state = Arc::new(Mutex::new(SdrMetrics {
            frequency:           cfg.radio.frequency_hz,
            config_sample_rate:  cfg.radio.sample_rate,
            lna_gain:            cfg.radio.lna_gain,
            vga_gain:            cfg.radio.vga_gain,
            amp_enabled:         cfg.radio.amp_enabled,
            // all other fields unchanged:
            actual_sample_rate: 0,
            rx_enabled: false,
            // ... etc
            waterfall: crate::state::WaterfallBuffer::new(cfg.display.waterfall_max_rows),
            // ...
        }));
```

Replace only the five radio fields and `waterfall` — leave all other fields as-is.

- [ ] **Log startup errors** — add after the existing `push_log` block (the one
  that logs `"Connected: ..."` and `"Firmware: ..."`):

```rust
        {
            let names = ["frequency", "sample rate", "LNA gain", "VGA gain", "amp"];
            let mut m = state.lock().unwrap();
            for (result, name) in startup_results.iter().zip(names.iter()) {
                if let Err(e) = result {
                    m.push_log(format!("Startup: failed to set {}: {}", name, e));
                }
            }
        }
```

- [ ] **Apply active preset from config** — after `let engine = ...`:

```rust
        engine.set_preset(&cfg.display.active_preset);
```

- [ ] **Add `config_path` to `Ok(Self { ... })`**:

```rust
        Ok(Self {
            state,
            device,
            rx_ctx,
            config_path,
            events: EventStream::new(Duration::from_millis(100)),
            show_help: false,
            engine,
        })
```

- [ ] **Add `save_config()` helper** — add as a method on `App`, before `pub fn run`:

```rust
    fn save_config(&self) {
        let Some(path) = &self.config_path else { return };
        // Extract values while holding the lock, then drop before file I/O
        let (freq, rate, lna, vga, amp, wf_rows) = {
            let m = self.state.lock().unwrap();
            (m.frequency, m.config_sample_rate, m.lna_gain,
             m.vga_gain, m.amp_enabled, m.waterfall.max_rows)
        };
        let cfg = AppConfig {
            radio: RadioConfig {
                frequency_hz: freq,
                sample_rate:  rate,
                lna_gain:     lna,
                vga_gain:     vga,
                amp_enabled:  amp,
            },
            display: DisplayConfig {
                active_preset:      self.engine.active_preset().to_string(),
                waterfall_max_rows: wf_rows,
            },
        };
        let _ = cfg.save(path);
    }
```

- [ ] **Update the `'q'` key handler** in `run()` to save before returning:

```rust
                            KeyCode::Char('q') => {
                                self.save_config();
                                return Ok(());
                            }
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Run `cargo test`**. Expected: all tests pass.

---

## Step 5 — Final validation

```bash
cargo build --release
cargo test
cargo clippy -- -D warnings
```

Expected: zero errors, zero warnings, all tests pass.

**Manual test checklist:**

- [ ] `sdrtop --help` prints usage with `--frequency`, `--lna`, `--vga`, `--config`
- [ ] `sdrtop --frequency 433920000 --lna 32` starts with those settings visible
  in the telemetry panel
- [ ] First run (no config file): `~/.config/sdrtop/config.toml` is created on exit
  with default values
- [ ] Change frequency to 433 MHz (press `f`, type `433`), quit with `q`,
  restart → frequency is 433 MHz on startup
- [ ] Change preset to `spectrum` (press `3`), quit, restart → spectrum preset
  is active on startup
- [ ] Corrupt `~/.config/sdrtop/config.toml` (e.g. write `garbage`), restart →
  app starts with defaults; warning printed to stderr before TUI launches
- [ ] `sdrtop --config /tmp/myconfig.toml` uses that path; creates it on quit
- [ ] All Phase 5–9 keys still work: gains, frequency, presets, waterfall pause
