# Phase 12 — PortaPack / Mayhem Integration: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)

**Goal:** Detect a PortaPack Mayhem device connected alongside the HackRF and display
live telemetry (Mayhem firmware version, platform model, RTC clock) in a dedicated panel.
A background thread polls the device over its USB serial interface; the panel is hidden
when no PortaPack is present.

**Prerequisite:** Phase 11 complete.

---

## Protocol background

PortaPack Mayhem exposes a USB CDC/ACM interface (Linux: `/dev/ttyACM*`) running a
text shell alongside the HackRF RF interface. The two interfaces are independent —
libhackrf already holds the RF interface; the serial interface is free to open
separately.

**Wire format:**

```
host → device:   <command>\r
device → host:   <response lines>\r\n
                 ok\r\n          ← every successful response ends here
```

**Commands used in this phase:**

| Command | Response | Purpose |
|---|---|---|
| `info\r` | Several lines + `ok\r\n` | Version, platform; used for detection |
| `rtcget\r` | Datetime string + `ok\r\n` | Device RTC |

**`info` response (Mayhem ≥ 2.x):**
```
Mayhem: 2.1.0
HackRF: 2024.02.1
CPLD: 46
Platform: PortaPack H2
ok
```
Detection: response contains the string `"Mayhem"`.
Parsing: line starting with `"Mayhem:"` → version; line starting with `"Platform:"` → model.

**`rtcget` response:**
```
20241015103045
ok
```
Format: `YYYYMMDDHHmmss` (14 digits). Reformat as `YYYY-MM-DD HH:mm:ss` for display.

---

## Correctness notes

**No blocking on the render loop.** The PortaPackWorker runs on a dedicated `std::thread`
(same pattern as FftWorker). It never holds `SdrMetrics` mutex while doing serial I/O.

**Graceful absence.** If no `/dev/ttyACM*` device is found, or if `info` doesn't return
`"Mayhem"`, `portapack.connected` stays `false` and the worker retries every 5 seconds.
The panel shows "No PortaPack detected" in this state. No error, no crash.

**Serial timeouts.** Each `port.read()` has a 2-second OS-level timeout (set at open
time). A hung device will not block the worker indefinitely.

**Reconnect.** If a read/write fails on an open port, the inner loop breaks; the outer
loop sleeps 5 seconds and re-scans `/dev/ttyACM*`. This handles unplug/replug.

**`serialport` is blocking.** `port.read()` blocks up to the timeout. Running the
worker on `std::thread::spawn` is mandatory — never call serial I/O inside a tokio task.

---

## Dependency order

```
Cargo.toml          add serialport = "4"
    ↓
src/state.rs        PortaPackState struct + SdrMetrics field
    ↓
src/portapack.rs    find_portapack() + read_response() + PortaPackWorker
    ↓
src/app.rs          spawn PortaPackWorker thread + '6' preset key
    ↓
src/ui/portapack_panel.rs   PortaPackPanel
src/ui/mod.rs       register module + pub use
src/config.rs       "portapack" preset in default_config()
src/ui/overlay.rs   add [6] line
```

---

## Step 1 — `serialport` dependency + `PortaPackState` + unit tests

**Files:** `Cargo.toml`, `src/state.rs`

- [ ] **Add to `Cargo.toml` `[dependencies]`:**

```toml
serialport = "4"
```

- [ ] **Add `PortaPackState` struct** to `src/state.rs` — insert before `SdrMetrics`:

```rust
#[derive(Clone, Debug)]
pub struct PortaPackState {
    pub connected: bool,
    pub port_path: String,
    pub mayhem_version: String,
    pub platform: String,
    pub rtc_time: String,
}

impl Default for PortaPackState {
    fn default() -> Self {
        Self {
            connected: false,
            port_path: String::new(),
            mayhem_version: String::new(),
            platform: String::new(),
            rtc_time: String::new(),
        }
    }
}
```

- [ ] **Add `portapack` field to `SdrMetrics`** — insert after `pub waterfall`:

```rust
    pub portapack: PortaPackState,
```

- [ ] **Add `portapack: PortaPackState::default()` to the `SdrMetrics` initializer**
  in `src/app.rs` (after the `waterfall:` line):

```rust
            portapack: crate::state::PortaPackState::default(),
```

- [ ] **Add 3 unit tests** to the existing `mod tests` block in `src/state.rs`:

```rust
    #[test]
    fn portapack_default_is_disconnected() {
        let pp = PortaPackState::default();
        assert!(!pp.connected);
        assert!(pp.mayhem_version.is_empty());
    }

    #[test]
    fn rtc_reformat_from_14_digits() {
        let raw = "20241015103045";
        let formatted = if raw.len() == 14 {
            format!("{}-{}-{} {}:{}:{}",
                &raw[0..4], &raw[4..6], &raw[6..8],
                &raw[8..10], &raw[10..12], &raw[12..14])
        } else {
            raw.to_string()
        };
        assert_eq!(formatted, "2024-10-15 10:30:45");
    }

    #[test]
    fn info_parser_extracts_version_and_platform() {
        let response = "Mayhem: 2.1.0\r\nHackRF: 2024.02.1\r\nPlatform: PortaPack H2\r\nok\r\n";
        let version = response.lines()
            .find(|l| l.starts_with("Mayhem:"))
            .and_then(|l| l.splitn(2, ':').nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let platform = response.lines()
            .find(|l| l.starts_with("Platform:"))
            .and_then(|l| l.splitn(2, ':').nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        assert_eq!(version, "2.1.0");
        assert_eq!(platform, "PortaPack H2");
    }
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Run `cargo test state::tests`**. Expected: all tests pass (6 existing + 3 new).

---

## Step 2 — `src/portapack.rs`

**Files:** `src/portapack.rs` (new), `src/main.rs` (add `mod portapack;`)

- [ ] **Create `src/portapack.rs`:**

```rust
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::state::{PortaPackState, SdrMetrics};

/// Read bytes from port until the response ends with `ok\r\n` or timeout/error.
/// Returns `None` on timeout or read error.
fn read_response(port: &mut dyn serialport::SerialPort) -> Option<String> {
    let mut buf = Vec::with_capacity(256);
    let mut byte = [0u8; 1];
    // Read at most 8 KB; the 2-second OS timeout guards against infinite blocking
    for _ in 0..8192 {
        match port.read(&mut byte) {
            Ok(1) => {
                buf.push(byte[0]);
                if buf.ends_with(b"ok\r\n") {
                    return Some(String::from_utf8_lossy(&buf).into_owned());
                }
            }
            _ => return None,
        }
    }
    None
}

/// Send a command and return the response, or `None` on I/O error.
fn send_command(port: &mut dyn serialport::SerialPort, cmd: &[u8]) -> Option<String> {
    port.write_all(cmd).ok()?;
    read_response(port)
}

/// Scan `/dev/ttyACM*` devices and return the first that responds to `info`
/// with a Mayhem firmware response, together with its path.
fn find_portapack() -> Option<(Box<dyn serialport::SerialPort>, String)> {
    let mut entries: Vec<_> = std::fs::read_dir("/dev")
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("ttyACM"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path().to_string_lossy().into_owned();
        if let Ok(mut port) = serialport::new(&path, 115_200)
            .timeout(Duration::from_secs(2))
            .open()
        {
            if let Some(resp) = send_command(&mut *port, b"info\r") {
                if resp.contains("Mayhem") {
                    return Some((port, path));
                }
            }
        }
    }
    None
}

fn parse_info(resp: &str) -> (String, String) {
    let version = resp
        .lines()
        .find(|l| l.starts_with("Mayhem:"))
        .and_then(|l| l.splitn(2, ':').nth(1))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let platform = resp
        .lines()
        .find(|l| l.starts_with("Platform:"))
        .and_then(|l| l.splitn(2, ':').nth(1))
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    (version, platform)
}

fn parse_rtc(resp: &str) -> String {
    let raw = resp
        .lines()
        .find(|l| !l.trim().is_empty() && *l != "ok")
        .unwrap_or("")
        .trim();
    if raw.len() == 14 && raw.chars().all(|c| c.is_ascii_digit()) {
        format!(
            "{}-{}-{} {}:{}:{}",
            &raw[0..4], &raw[4..6], &raw[6..8],
            &raw[8..10], &raw[10..12], &raw[12..14]
        )
    } else {
        raw.to_string()
    }
}

pub struct PortaPackWorker {
    pub state: Arc<Mutex<SdrMetrics>>,
}

impl PortaPackWorker {
    pub fn run(self) {
        loop {
            match find_portapack() {
                None => {
                    std::thread::sleep(Duration::from_secs(5));
                }
                Some((mut port, path)) => {
                    // Parse the info response we already have buffered — re-query
                    let (version, platform) = match send_command(&mut *port, b"info\r") {
                        Some(resp) => parse_info(&resp),
                        None => (String::new(), String::new()),
                    };

                    if let Ok(mut m) = self.state.lock() {
                        m.portapack = PortaPackState {
                            connected: true,
                            port_path: path.clone(),
                            mayhem_version: version,
                            platform,
                            rtc_time: String::new(),
                        };
                        m.push_log(format!("PortaPack detected: {}", path));
                    }

                    // Poll loop: refresh RTC every 5 seconds
                    loop {
                        std::thread::sleep(Duration::from_secs(5));
                        match send_command(&mut *port, b"rtcget\r") {
                            Some(resp) => {
                                let rtc = parse_rtc(&resp);
                                if let Ok(mut m) = self.state.lock() {
                                    m.portapack.rtc_time = rtc;
                                }
                            }
                            None => {
                                // Port died — disconnect and re-scan
                                if let Ok(mut m) = self.state.lock() {
                                    m.portapack = PortaPackState::default();
                                    m.push_log("PortaPack disconnected");
                                }
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_info_extracts_fields() {
        let resp = "Mayhem: 2.1.0\r\nHackRF: 2024.02.1\r\nPlatform: PortaPack H2\r\nok\r\n";
        let (version, platform) = parse_info(resp);
        assert_eq!(version, "2.1.0");
        assert_eq!(platform, "PortaPack H2");
    }

    #[test]
    fn parse_info_missing_fields_returns_empty() {
        let resp = "some unknown response\r\nok\r\n";
        let (version, platform) = parse_info(resp);
        assert!(version.is_empty());
        assert!(platform.is_empty());
    }

    #[test]
    fn parse_rtc_14_digit_format() {
        let resp = "20241015103045\r\nok\r\n";
        assert_eq!(parse_rtc(resp), "2024-10-15 10:30:45");
    }

    #[test]
    fn parse_rtc_unexpected_format_passes_through() {
        let resp = "2024/10/15 10:30:45\r\nok\r\n";
        let result = parse_rtc(resp);
        assert!(!result.is_empty());
    }
}
```

- [ ] **Add `mod portapack;` to `src/main.rs`** — insert alphabetically with the other mods:

```rust
mod portapack;
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Run `cargo test portapack::tests`**. Expected: 4 tests pass.

---

## Step 3 — `App` integration

**Files:** `src/app.rs`

- [ ] **Add import** at the top of `src/app.rs`:

```rust
use crate::portapack::PortaPackWorker;
```

- [ ] **Spawn PortaPackWorker** — add after the FftWorker spawn block (the
  `std::thread::spawn(move || { FftWorker::new(...).run(); });` block):

```rust
        // PortaPack worker: serial I/O is blocking, must run on its own OS thread
        let pp_state = Arc::clone(&state);
        std::thread::spawn(move || {
            PortaPackWorker { state: pp_state }.run();
        });
```

- [ ] **Add `'6'` key handler** in the `InputMode::Normal` match arm, after the `'5'` arm:

```rust
                            KeyCode::Char('7') => {
                                self.engine.set_preset("portapack");
                                self.state.lock().unwrap().push_log("Preset: portapack");
                            }
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 4 — `PortaPackPanel`

**Files:** `src/ui/portapack_panel.rs` (new)

- [ ] **Create `src/ui/portapack_panel.rs`:**

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct PortaPackPanel;

impl Panel for PortaPackPanel {
    fn name(&self) -> &'static str {
        "portapack"
    }

    fn min_size(&self) -> (u16, u16) {
        (30, 6)
    }

    fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        let pp = &state.portapack;

        let block = Block::default()
            .title(if pp.connected {
                " PortaPack Mayhem ● "
            } else {
                " PortaPack Mayhem ○ "
            })
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if pp.connected {
                Color::Green
            } else {
                Color::DarkGray
            }));

        if !pp.connected {
            f.render_widget(
                Paragraph::new("No PortaPack detected\n\nScanning /dev/ttyACM*…")
                    .block(block)
                    .style(Style::default().fg(Color::DarkGray)),
                area,
            );
            return;
        }

        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let label = Style::default().fg(Color::DarkGray);
        let value = Style::default().fg(Color::White);

        let port_line = format!("Port    {}", pp.port_path);
        let ver_line  = format!("Mayhem  {}", pp.mayhem_version);
        let plat_line = format!("Model   {}", pp.platform);

        f.render_widget(Paragraph::new(port_line).style(label), rows[0]);
        f.render_widget(Paragraph::new(ver_line).style(value), rows[1]);
        f.render_widget(Paragraph::new(plat_line).style(value), rows[2]);

        if !pp.rtc_time.is_empty() {
            let rtc_line = format!("RTC     {}", pp.rtc_time);
            // rows[3] is Min(0) — only render if we have the space
            if rows[3].height > 0 {
                f.render_widget(Paragraph::new(rtc_line).style(value), rows[3]);
            }
        }
    }
}
```

---

## Step 5 — Register panel + preset + overlay

**Files:** `src/ui/mod.rs`, `src/config.rs`, `src/ui/overlay.rs`, `src/app.rs`

- [ ] **Add module and re-export** in `src/ui/mod.rs`:

```rust
pub mod portapack_panel;
pub use portapack_panel::PortaPackPanel;
```

Insert `pub mod portapack_panel;` alphabetically with the other `pub mod` lines,
and `pub use portapack_panel::PortaPackPanel;` with the `pub use` lines.

- [ ] **Register `PortaPackPanel`** in `src/app.rs` — add after the
  `registry.register(ui::WaterfallPanel::new());` line:

```rust
        registry.register(ui::PortaPackPanel);
```

- [ ] **Add `portapack` preset** to `LayoutConfig::default_config()` in `src/config.rs`
  — insert before the `let mut presets = HashMap::new();` line:

```rust
        let portapack = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),     position: Top,    height: Some(3), width_pct: None     },
                PanelSpec { name: "portapack".into(),  position: Left,   height: None,    width_pct: Some(50) },
                PanelSpec { name: "telemetry".into(),  position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "log".into(),        position: Bottom, height: Some(7), width_pct: None     },
                PanelSpec { name: "footer".into(),     position: Bottom, height: Some(3), width_pct: None     },
            ],
        };
```

And insert into the `presets` map:

```rust
        presets.insert("portapack".into(), portapack);
```

- [ ] **Update the help overlay** in `src/ui/overlay.rs` — replace the `[5]` line with:

```rust
 [5]        Preset: spectrum+waterfall\n\
 [6]        Preset: portapack\n\
```

And update the `centered_rect` call from height `21` to `22`:

```rust
    let area = centered_rect(52, 22, f.size());
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Run `cargo test`**. Expected: all tests pass (35 total).

---

## Step 6 — Final validation

```bash
cargo build --release
cargo test
cargo clippy -- -D warnings
```

Expected: zero errors, zero warnings, all tests pass.

**Manual test checklist (without hardware):**

- [ ] `sdrtop --help` still shows correct usage
- [ ] App starts without a PortaPack: `portapack` panel shows "No PortaPack detected"
- [ ] `[6]` switches to portapack preset; panel visible; no crash
- [ ] `[?]` help overlay lists `[6]  Preset: portapack`
- [ ] `[p]` cycles through all presets including portapack
- [ ] All Phase 5–10 keys still work

**Manual test checklist (with PortaPack):**

- [ ] Panel shows green `●` indicator, port path, Mayhem version, model
- [ ] RTC time appears within 10 seconds of startup
- [ ] Unplug PortaPack mid-session → panel reverts to "No PortaPack detected" within ~5 s
- [ ] Replug → reconnects automatically within 10 s
