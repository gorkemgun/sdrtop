# sdrtop — Roadmap to btop-level Quality

## Vision

`sdrtop` is a terminal-based SDR monitor in the spirit of `btop`: visually rich,
fully interactive, and genuinely useful as a daily driver. The end state is an app
that an RF engineer opens instead of `hackrf_info` + `gqrx` + a scratchpad — one
tool that shows everything, lets you tune everything, and gets out of the way.

**Current Focus:** HackRF One and PortaPack H1/H2 with Mayhem firmware (primary).
While these are the immediate priority based on available hardware, the architecture 
aims for future extensibility to other SDR platforms (e.g., RTL-SDR, LimeSDR, Airspy).

---

## Current Status

| Phase | Status |
|---|---|
| 1 — Device discovery & basic info | ✅ Done |
| 2 — Telemetry polling & USB throughput | ✅ Done |
| 3 — TUI dashboard (gauges, sparkline, log, shortcuts) | ✅ Done |

---

## Technology Stack

| Concern | Choice | Notes |
|---|---|---|
| Language | Rust stable | |
| TUI | `ratatui 0.26+` | layout, widgets, Braille canvas |
| Hardware FFI | `libhackrf` via `pkg-config` | custom FFI (bypasses broken 0.0.1 crate) |
| Async runtime | `tokio` | background polling & FFT task |
| FFT | `rustfft 6` | pure-Rust, no C dependency |
| Config | `toml 0.8` + `serde 1` | `~/.config/sdrtop/config.toml` |
| CLI args | `clap 4` (derive feature) | |
| Channels | `crossbeam-channel 0.5` | lock-free sample handoff |

---

## Phase 4 — Architecture Refactor

**Goal:** `main.rs` becomes an entry point only. Split into focused modules before
adding more features. Every future phase has a clean home; no file exceeds ~200 lines.

### Target module layout

```
src/
  main.rs               entry point: init, hand off to App::run()
  app.rs                App struct + run() event loop
  event.rs              AppEvent enum (Key, Tick, HwEvent)
  state.rs              SdrMetrics, AppState, constants
  config.rs             Config struct — load/save (Phase 8)
  hardware/
    mod.rs              pub use device::Device;
    ffi.rs              raw extern "C" block only
    device.rs           safe Device wrapper + all pub methods
    buffer.rs           sample ring buffer (Phase 6)
  ui/
    mod.rs              pub fn draw(frame, app)
    layout.rs           chunk helpers, panel size constants
    header.rs           fn render_header(frame, area, state)
    telemetry.rs        fn render_telemetry(frame, area, state)
    gains.rs            fn render_gains(frame, area, state)
    sparkline.rs        fn render_sparkline(frame, area, state)
    spectrum.rs         fn render_spectrum(frame, area, state) (Phase 6)
    waterfall.rs        fn render_waterfall(frame, area, state) (Phase 7)
    log.rs              fn render_log(frame, area, state)
    footer.rs           fn render_footer(frame, area, state)
    overlay.rs          fn render_help_overlay(frame, state) (Phase 5)
```

### Steps

**4.1 — Create directory skeleton**
- [ ] `mkdir -p src/hardware src/ui`
- [ ] Create empty `src/hardware/mod.rs` and `src/ui/mod.rs`

**4.2 — Extract `src/hardware/ffi.rs`**
- [ ] Move the raw `extern "C"` block and all `#[repr(C)]` structs
      (`hackrf_transfer`, `HackrfDeviceList`, `ReadPartidSerialno`,
      `HackrfTransferCallback`) out of `main.rs` into `ffi.rs`
- [ ] `pub use` them from `hardware/mod.rs`
- [ ] `cargo build` — must pass

**4.3 — Extract `src/hardware/device.rs`**
- [ ] Move `Device` struct, all `impl Device` methods, and `impl Drop for Device`
      into `device.rs`
- [ ] Move `rx_callback` extern fn into `device.rs`
- [ ] `cargo build` — must pass

**4.4 — Extract `src/state.rs`**
- [ ] Move `SdrMetrics` struct, `impl SdrMetrics` (`push_log`, `reset_to_defaults`)
- [ ] Move all constants (`THROUGHPUT_HISTORY_LEN`, `LOG_MAX_ENTRIES`,
      `DEFAULT_*`) into `state.rs`
- [ ] Add `pub struct AppState` wrapping `Arc<Mutex<SdrMetrics>>` — this will
      grow in later phases
- [ ] `cargo build` — must pass

**4.5 — Extract `src/event.rs`**
- [ ] Define:
  ```rust
  pub enum AppEvent {
      Key(crossterm::event::KeyEvent),
      Tick,
  }
  ```
- [ ] Implement `EventStream`: a struct that spawns a thread, polls crossterm
      events, and sends `AppEvent` on a `crossbeam_channel::Sender<AppEvent>`
      with a configurable tick rate (default 100 ms)
- [ ] UI loop uses the receiver instead of `event::poll` directly
- [ ] `cargo build` — must pass

**4.6 — Extract UI panel functions**
- [ ] Create `src/ui/header.rs`: `pub fn render(f, area, board_name, fw, serial)`
- [ ] Create `src/ui/telemetry.rs`: `pub fn render(f, area, m: &SdrMetrics, board_name, serial)`
- [ ] Create `src/ui/gains.rs`: `pub fn render(f, area, m: &SdrMetrics)`
      — contains LNA, VGA, sample-rate gauges and sparkline
- [ ] Create `src/ui/log.rs`: `pub fn render(f, area, m: &SdrMetrics)`
- [ ] Create `src/ui/footer.rs`: `pub fn render(f, area, mode: InputMode)`
      — `InputMode` is `Normal` for now, extended in Phase 5
- [ ] Create `src/ui/layout.rs`: `pub fn build_chunks(size) -> Chunks` struct
      holding all split areas (header, body_left, body_right, gain_*,
      sparkline, log, footer)
- [ ] Create `src/ui/mod.rs`: `pub fn draw(f, app)` that calls all of the above
- [ ] `cargo build` — must pass

**4.7 — Create `src/app.rs`**
- [ ] Define `App` struct:
  ```rust
  pub struct App {
      state: Arc<Mutex<SdrMetrics>>,
      device: Arc<hardware::Device>,
      board_name: String,
      fw_version: String,
      serial: String,
      events: EventStream,
  }
  ```
- [ ] Move `main()` body (device open, metric init, tokio::spawn polling task)
      into `App::new() -> Result<Self>`
- [ ] Move the `run_app()` loop into `App::run(&mut self) -> io::Result<()>`
      using `EventStream` receiver
- [ ] `main.rs` becomes: open terminal → `App::new()?.run()` → restore terminal

**4.8 — Final validation**
- [ ] `cargo build --release` — zero warnings
- [ ] `cargo clippy -- -D warnings` — zero findings
- [ ] Run the binary with a real HackRF; confirm behaviour is identical to pre-refactor

---

## Phase 5 — Interactive Controls

**Goal:** Every parameter visible in the UI can be changed live from the keyboard.
Hardware is called immediately; the display reflects the new value within one render frame.

### Full keybinding table

| Key | Action | HackRF call |
|---|---|---|
| `l` | LNA gain +8 dB (max 40) | `hackrf_set_lna_gain` |
| `L` (Shift+L) | LNA gain −8 dB (min 0) | `hackrf_set_lna_gain` |
| `v` | VGA gain +2 dB (max 62) | `hackrf_set_vga_gain` |
| `V` (Shift+V) | VGA gain −2 dB (min 0) | `hackrf_set_vga_gain` |
| `a` | Toggle AMP enable | `hackrf_set_amp_enable` |
| `s` | Cycle sample rate (2/4/8/10/12.5/16/20 Msps) | `hackrf_set_sample_rate` |
| `f` | Enter frequency input mode | — |
| `Enter` (in freq mode) | Confirm frequency, call hardware | `hackrf_set_freq` |
| `Esc` (in freq mode) | Cancel, return to Normal mode | — |
| `r` | Reset all to defaults | all setters |
| `?` | Toggle help overlay | — |
| `q` | Quit | — |
| `Space` | Toggle RX streaming | `hackrf_start_rx` / `hackrf_stop_rx` |

### Steps

**5.1 — Add `InputMode` to state**
- [ ] In `state.rs`, add:
  ```rust
  #[derive(Clone, PartialEq)]
  pub enum InputMode { Normal, FrequencyInput }
  ```
- [ ] Add `input_mode: InputMode` and `freq_input_buf: String` to `SdrMetrics`
- [ ] Update `#[derive(Clone)]` to handle these new fields

**5.2 — Wire gain keys**
- [ ] In `app.rs` event handler, match `KeyCode::Char('l')`:
  - read current `lna_gain`, add 8, clamp to 40, write back
  - call `device.set_lna_gain(new_val)`, push result to log
- [ ] Match `KeyCode::Char('L')` (check `modifiers == KeyModifiers::SHIFT`):
  - subtract 8, clamp to 0, same pattern
- [ ] Same for `v` / `V` with VGA (step 2 dB, max 62)
- [ ] `cargo build` — must pass
- [ ] Manual test: press `l`/`L`, observe LNA gauge move and log entry appear

**5.3 — Wire AMP toggle**
- [ ] Match `KeyCode::Char('a')`:
  - flip `amp_enabled`
  - call `device.set_amp_enable(new_val)`, push result to log
- [ ] Manual test: press `a`, confirm AMP status line toggles ON/OFF

**5.4 — Wire sample rate cycle**
- [ ] Define `SAMPLE_RATE_STEPS: &[f64] = &[2e6, 4e6, 8e6, 10e6, 12.5e6, 16e6, 20e6]`
      in `state.rs`
- [ ] Match `KeyCode::Char('s')`:
  - find current `config_sample_rate` in `SAMPLE_RATE_STEPS`, advance index (wraps)
  - call `device.set_sample_rate(new_val)`, update `config_sample_rate`, push log

**5.5 — Implement frequency input mode**
- [ ] Match `KeyCode::Char('f')` when `input_mode == Normal`:
  - set `input_mode = FrequencyInput`, clear `freq_input_buf`
- [ ] In `FrequencyInput` mode, route all printable digit / `.` keys to
      `freq_input_buf.push(ch)`, route `Backspace` to `freq_input_buf.pop()`
- [ ] On `Enter`:
  - parse `freq_input_buf` as `f64` (MHz)
  - validate range 1–7250 MHz (HackRF hardware limits)
  - on valid: convert to Hz (`u64`), call `device.set_frequency(hz)`,
    update `state.frequency`, push log "Frequency set to X MHz"
  - on invalid: push log "Invalid frequency: must be 1–7250 MHz"
  - set `input_mode = Normal`
- [ ] On `Esc`: clear buf, set `input_mode = Normal`

**5.6 — Update footer widget for input mode**
- [ ] In `ui/footer.rs`, check `input_mode`:
  - `Normal`: render keybind hints as before
  - `FrequencyInput`: render `" Frequency (MHz): [<buf>_] | Enter = confirm | Esc = cancel "`
    with cursor represented as `_`

**5.7 — Help overlay**
- [ ] Create `src/ui/overlay.rs`:
  ```rust
  pub fn render_help(f: &mut Frame, all_keys: &[(key, description)])
  ```
  - centered `Clear` + bordered `Paragraph` listing every keybinding
- [ ] In `App`, add `show_help: bool` field
- [ ] Match `KeyCode::Char('?')` to toggle `show_help`
- [ ] In `ui/mod.rs` `draw()`: if `show_help`, call `render_help` last (on top)
- [ ] Manual test: press `?`, overlay appears; press again, disappears

**5.8 — End-to-end validation**
- [ ] Every key in the table above exercised manually
- [ ] All hardware calls return `Ok`; any `Err` appears in the log panel, never crashes
- [ ] `cargo clippy -- -D warnings` — zero findings

---

## Phase 6 — FFT Spectrum Analyzer

**Goal:** A live, full-width spectrum display on a Braille canvas — the feature that
makes `sdrtop` genuinely useful for RF work instead of just pretty.

### Data pipeline

```
RX callback (libhackrf thread)
   │  raw IQ bytes pushed into crossbeam channel (bounded, drops oldest on full)
   ▼
FftWorker (tokio task)
   │  reads N samples, applies window function, runs rustfft
   │  converts magnitude to dBFS, runs EMA, computes peak-hold
   │  sends FftFrame { bins: Vec<f32> } on a second bounded channel
   ▼
UI render loop
   │  receives latest FftFrame (non-blocking, uses previous if none ready)
   ▼
SpectrumWidget → Canvas → Braille dots
```

The UI never waits for FFT. If the FFT worker is behind, the UI re-renders
the last good frame and shows a stale-frame indicator.

### FftFrame spec

```rust
pub struct FftFrame {
    pub bins_dbfs: Vec<f32>,   // length = fft_size, ordered low→high freq
    pub peak_hold: Vec<f32>,   // same length, decaying peak
    pub noise_floor: f32,      // running average of bottom 10% of bins
    pub center_freq_hz: u64,
    pub sample_rate: f64,
    pub stale: bool,           // true if this frame is older than 500 ms
}
```

### Steps

**6.1 — Add dependencies**
- [ ] Add to `Cargo.toml`:
  ```toml
  rustfft = "6"
  crossbeam-channel = "0.5"
  num-complex = "0.4"
  ```
- [ ] `cargo build` — must pass

**6.2 — Sample ring buffer (`src/hardware/buffer.rs`)**
- [ ] Define `SampleBuffer`:
  - wraps a `crossbeam_channel::Sender<Vec<u8>>`
  - channel bounded at 4 messages (≈ 4 × callback buffer, ~1 M samples)
- [ ] `SampleBuffer::push(&self, data: &[u8])` — sends a clone; on full channel
      pops the oldest by doing a non-blocking `recv` first, then `send`
- [ ] `SampleBuffer::receiver() -> Receiver<Vec<u8>>` — returns the other half
- [ ] Update `rx_callback` to call `SampleBuffer::push` instead of accumulating
      in `SdrMetrics.bytes_since_last_poll` — throughput counting moves to
      the FFT worker (it already has the byte count from the received Vec)

**6.3 — FFT worker (`src/fft.rs`)**
- [ ] Define `FftWorker` struct:
  ```rust
  pub struct FftWorker {
      samples_rx: Receiver<Vec<u8>>,
      frame_tx: Sender<FftFrame>,
      fft_size: usize,
      window: WindowFn,
      ema_alpha: f32,
  }
  ```
- [ ] Implement window functions in `src/dsp.rs`:
  - `hann(size: usize) -> Vec<f32>`
  - `hamming(size: usize) -> Vec<f32>`
  - `blackman(size: usize) -> Vec<f32>`
  - `pub enum WindowFn { Hann, Hamming, Blackman }`
- [ ] Implement `FftWorker::run(self)` as an async loop:
  1. accumulate raw bytes into a local `Vec<u8>` until `len >= fft_size * 2`
     (2 bytes per complex IQ sample)
  2. convert bytes to `Vec<Complex<f32>>`: `i = byte as f32 / 128.0 - 1.0`
  3. apply window function element-wise
  4. run `rustfft` in-place
  5. compute magnitude: `20 * log10(|z| / fft_size)` → dBFS
  6. shift output so DC is at index 0 → center of display (fftshift)
  7. apply EMA: `bin = alpha * new + (1-alpha) * prev`
  8. update peak-hold: `peak[i] = max(peak[i] - decay, bin[i])`
  9. compute noise floor: mean of bottom 10% of bin values
  10. send `FftFrame` on `frame_tx`; if channel full, drop frame (non-blocking `try_send`)

**6.4 — Wire FftWorker into App**
- [ ] In `App::new()`, create `SampleBuffer`, give `Sender` to `rx_callback` context,
      give `Receiver` to `FftWorker`
- [ ] Spawn `FftWorker::run()` as a `tokio::task`
- [ ] Add `fft_rx: Receiver<FftFrame>` to `App`; store latest received frame in
      `App.last_fft_frame: Option<FftFrame>`
- [ ] In the render loop, do a non-blocking `fft_rx.try_recv()` before `draw()`;
      update `last_fft_frame` if a new frame arrived

**6.5 — Spectrum widget (`src/ui/spectrum.rs`)**
- [ ] Implement `pub fn render(f, area, frame: Option<&FftFrame>, center_hz, sr)`
- [ ] Use `ratatui::widgets::canvas::Canvas`:
  - x range: 0.0 ..= 1.0 (normalized bin index)
  - y range: `db_min ..= db_max` (configurable, default −120..0)
  - draw a filled bar for each bin using Braille dots (`.line()` from bottom of
    y-range to bin value)
- [ ] Draw peak-hold as a separate line in a dimmer color
- [ ] Draw noise floor as a dashed horizontal line
- [ ] Render frequency axis: 5 equally-spaced labels in MHz below the canvas
- [ ] Render dBFS axis: 5 labels on the left side
- [ ] If `frame.stale`, tint the entire widget grey and add `[STALE]` to title
- [ ] If `frame` is `None`, render an empty canvas with "Waiting for RX…" centered

**6.6 — Integrate spectrum into layout**
- [ ] Update `ui/layout.rs` to add a `spectrum` area above the existing body
      (default height 14 rows, configurable)
- [ ] Update `ui/mod.rs` `draw()` to call `spectrum::render`
- [ ] Add `n` key to cycle FFT size: `[1024, 2048, 4096]`
  - send new size to `FftWorker` via a `watch::Sender<usize>` (tokio watch channel)
  - worker re-creates its planner and window on size change
- [ ] Add `w` key to cycle window function: Hann → Hamming → Blackman → Hann

**6.7 — Benchmark**
- [ ] Run with a real HackRF at 20 Msps; verify FFT frame rate ≥ 10 fps in
      the log panel (add a frame counter to `FftFrame`)
- [ ] On Raspberry Pi 4 (if available): target ≥ 5 fps at 2048-point FFT
- [ ] `cargo build --release` — profile build must pass clean

---

## Phase 7 — Waterfall Display

**Goal:** A scrolling 2D spectrum history below the spectrum plot.
Every row is one FFT frame; newer frames push from the top, time flows downward.

### Color palette

| Terminal capability | Palette used |
|---|---|
| Truecolor (`COLORTERM=truecolor`) | 24-bit RGB gradient: `#000080` → `#00ff00` → `#ffff00` → `#ff0000` |
| 256-color | pre-computed 32-entry lookup into xterm-256 palette |
| 16-color fallback | 4 levels: black, dark blue, cyan, white |

### Steps

**7.1 — WaterfallBuffer**
- [ ] In `state.rs`, add:
  ```rust
  pub struct WaterfallBuffer {
      rows: VecDeque<Vec<f32>>,    // each Vec<f32> is one FftFrame's bins_dbfs
      max_rows: usize,
      paused: bool,
  }
  ```
- [ ] `WaterfallBuffer::push(&mut self, bins: Vec<f32>)` — adds row at front,
      pops back if `len > max_rows`; no-op if `paused`
- [ ] Add `waterfall: WaterfallBuffer` to `SdrMetrics`
- [ ] Update the FFT frame consumer in `app.rs`: on each new `FftFrame`, call
      `state.waterfall.push(frame.bins_dbfs.clone())`

**7.2 — Color palette**
- [ ] Create `src/palette.rs`:
  - `pub enum ColorDepth { True, Colors256, Colors16 }`
  - `pub fn detect() -> ColorDepth` — reads `COLORTERM` env var
  - `pub fn magnitude_to_color(db: f32, db_min: f32, db_max: f32, depth: ColorDepth) -> ratatui::style::Color`
    - normalizes `db` to 0.0–1.0, maps to palette color

**7.3 — WaterfallWidget**
- [ ] Create `src/ui/waterfall.rs`:
  `pub fn render(f, area, buf: &WaterfallBuffer, db_min, db_max, depth: ColorDepth)`
- [ ] Use `Canvas` with `Rectangle` shapes (each bin × each row = one rectangle)
      colored by `magnitude_to_color`
- [ ] Each rendered cell is 1 terminal column wide × 1 terminal row tall
      (skip Braille here — solid colored blocks give better visual density)
- [ ] If `buf.paused`, add `[PAUSED]` to widget title

**7.4 — Layout integration**
- [ ] Add `show_waterfall: bool` and `waterfall_height: u16` to `SdrMetrics`
- [ ] Update `ui/layout.rs`: if `show_waterfall`, split the spectrum area into
      spectrum (top portion) + waterfall (bottom portion) using configured heights
- [ ] Update `ui/mod.rs` to conditionally call `waterfall::render`

**7.5 — Keyboard controls**
- [ ] `w` key (already used for window function in Phase 6):
      reassign — use `w` to cycle display mode: Spectrum only → Both → Waterfall only
      Move window function to `W` (shift)
- [ ] `p` key: toggle `waterfall.paused`; push log "Waterfall paused / resumed"

**7.6 — Validation**
- [ ] At 80×24 terminal, 2048-point FFT, `Both` mode: confirm render stays ≥ 10 fps
- [ ] Verify palette degrades correctly: test with `COLORTERM` unset, then
      with `TERM=xterm` (16-color)

---

## Phase 8 — Configuration & Persistence

**Goal:** Settings survive restarts. The app opens in the same state as when it exited.

### Config file location

`$XDG_CONFIG_HOME/sdrtop/config.toml` (falls back to `~/.config/sdrtop/config.toml`)

### Full config schema

```toml
[device]
serial = ""              # empty = auto-select first found

[radio]
frequency_hz     = 2400000000
sample_rate      = 10000000.0
lna_gain         = 16
vga_gain         = 20
amp_enabled      = false
fft_size         = 2048
fft_window       = "hann"     # hann | hamming | blackman

[display]
spectrum_height  = 14
waterfall_rows   = 20
spectrum_db_min  = -120
spectrum_db_max  = 0
theme            = "default"  # default | gruvbox | nord | light
show_waterfall   = true

[keybindings]
# All keys can be overridden, e.g.:
# quit = "q"
# toggle_rx = " "
```

### Steps

**8.1 — Add dependencies**
- [ ] Add to `Cargo.toml`:
  ```toml
  serde = { version = "1", features = ["derive"] }
  toml  = "0.8"
  clap  = { version = "4", features = ["derive"] }
  ```

**8.2 — Define `Config` struct (`src/config.rs`)**
- [ ] Define nested structs `DeviceConfig`, `RadioConfig`, `DisplayConfig`,
      `KeybindingsConfig`, all with `#[derive(Serialize, Deserialize, Default)]`
- [ ] Define `Config` holding all four; also `#[derive(Serialize, Deserialize, Default)]`
- [ ] Add `#[serde(deny_unknown_fields)]` to catch typos; emit a log warning
      instead of hard-error (catch parse error, retry with a lenient parse)

**8.3 — Implement `Config::load_or_default(path: &Path) -> Config`**
- [ ] Check if the file exists; if not, return `Config::default()` (no error)
- [ ] Read file → `toml::from_str`; on error log a warning, return default
- [ ] Create parent dirs if they don't exist (`fs::create_dir_all`)

**8.4 — Implement `Config::save(&self, path: &Path) -> Result<()>`**
- [ ] Serialize to TOML string via `toml::to_string_pretty`
- [ ] Write atomically: write to `<path>.tmp`, then `fs::rename`

**8.5 — CLI args via clap**
- [ ] Define `Args` struct in `main.rs`:
  ```rust
  #[derive(clap::Parser)]
  struct Args {
      #[arg(long)] config: Option<PathBuf>,
      #[arg(long)] frequency: Option<f64>,   // MHz, overrides config
      #[arg(long)] lna: Option<u32>,
      #[arg(long)] vga: Option<u32>,
      #[arg(long)] serial: Option<String>,
  }
  ```
- [ ] In `main()`, parse `Args`, resolve config path, call `Config::load_or_default`
- [ ] Merge CLI args over config values (CLI wins)

**8.6 — Apply config to initial state**
- [ ] In `App::new()`, use config values to populate initial `SdrMetrics`
      (frequency, gains, fft_size, display settings)
- [ ] If `config.device.serial` is non-empty, pass it to `Device::open_by_serial()`
      (add this method to `device.rs`)

**8.7 — Save on exit**
- [ ] In `App::run()`, on `'q'` key: update `config.radio` from current `SdrMetrics`,
      call `config.save()`, then return
- [ ] On panic / signal exit: best-effort save (catch with `std::panic::set_hook`)

---

## Phase 9 — Multi-Device Support

**Goal:** Multiple HackRF devices monitored simultaneously; `Tab` switches focus.

### Data model change

```rust
// Before (Phase 4)
App { state: Arc<Mutex<SdrMetrics>>, device: Arc<Device>, … }

// After (Phase 9)
App { devices: Vec<DeviceHandle>, focused: usize, … }

struct DeviceHandle {
    device:     Arc<Device>,
    state:      Arc<Mutex<SdrMetrics>>,
    board_name: String,
    serial:     String,
    fw_version: String,
    online:     bool,
}
```

### Steps

**9.1 — Introduce `DeviceHandle`**
- [ ] Define `DeviceHandle` in `src/device_handle.rs`
- [ ] Refactor `App` to hold `Vec<DeviceHandle>` and `focused: usize`
- [ ] All UI render calls use `devices[focused]`
- [ ] All keyboard actions operate on `devices[focused]`
- [ ] `cargo build` — must pass, single-device behaviour unchanged

**9.2 — Open all connected devices at startup**
- [ ] In `App::new()`, enumerate all connected serials via `hackrf_device_list`
- [ ] Open each in sequence, spawn one polling task and one FFT worker per device
- [ ] If any device fails to open, log error and continue with the remaining ones
- [ ] If zero devices open successfully, exit with clear error message

**9.3 — Device list panel (`src/ui/device_list.rs`)**
- [ ] `pub fn render(f, area, devices: &[DeviceHandle], focused: usize)`
- [ ] Shows one row per device: `[*] HackRF One | S/N: abc123 | STREAMING`
  - `[*]` marks the focused device
  - Offline devices shown in red with `[OFFLINE]`
- [ ] Add `d` key to toggle this panel (side panel, ~30% width on the left)
- [ ] `Tab` / `Shift-Tab` advance/retreat `focused` index

**9.4 — Disconnect detection**
- [ ] Each polling task already calls `board.is_streaming()`; add a USB error check:
  - if `hackrf_is_streaming` returns a specific error code (not 0 or 1), mark
    `DeviceHandle.online = false`
  - push log entry: "Device <serial> disconnected"
  - stop the FFT worker for that device (send a shutdown signal via `CancellationToken`)
- [ ] UI: offline devices show a greyed-out panel with `[OFFLINE]` watermark

**9.5 — Reconnect detection**
- [ ] A global "device watcher" task polls `hackrf_device_list` every 2 seconds
- [ ] If a previously-offline serial reappears, call `Device::open_by_serial`,
      recreate `DeviceHandle`, restart polling and FFT tasks
- [ ] Push log: "Device <serial> reconnected"
- [ ] Test: unplug and replug a HackRF during a live session

---

## Phase 10 — PortaPack / Mayhem Integration

**Goal:** When a PortaPack with Mayhem firmware is connected, show Mayhem-specific
telemetry in an additional panel.

### Detection strategy

Mayhem firmware sets a distinct USB product string. On `Device::open`, read the
USB product string via `libusb` (already linked transitively through `libhackrf`).
If the string contains `"PortaPack"`, set `device.is_portapack = true`.

### Known Mayhem telemetry (USB vendor control transfers)

| Data | USB bRequest | Notes |
|---|---|---|
| Battery voltage (mV) | 0x10 | PortaPack H2 only |
| Battery percent | derived from voltage curve | |
| Active application | 0x11 | UTF-8 string, max 32 bytes |
| GPS fix + coordinates | 0x12 | lat/lon as fixed-point i32 |

### Steps

**10.1 — USB product string detection**
- [ ] Add `libusb-sys` or use the raw `libusb1-sys` already linked by `libhackrf`
- [ ] In `Device::open()`, after `hackrf_device_list_open`, read USB product string
      via `libusb_get_device_descriptor` + `libusb_get_string_descriptor_ascii`
- [ ] Set `Device.is_portapack: bool` based on product string
- [ ] `cargo build` — must pass

**10.2 — Vendor control transfer helper**
- [ ] Add `Device::vendor_read(request: u8, buf: &mut [u8]) -> Result<usize>`
      wrapping `libusb_control_transfer` with `LIBUSB_REQUEST_TYPE_VENDOR | LIBUSB_RECIPIENT_DEVICE | LIBUSB_ENDPOINT_IN`

**10.3 — PortaPack telemetry polling**
- [ ] In the polling task, if `device.is_portapack`:
  - read battery voltage (request 0x10), store in `SdrMetrics.battery_mv: Option<u32>`
  - read active app string (request 0x11), store in `SdrMetrics.active_app: Option<String>`
  - read GPS data (request 0x12), store in `SdrMetrics.gps: Option<GpsData>`
- [ ] `GpsData { lat: f64, lon: f64, fix: bool }`

**10.4 — PortaPack panel (`src/ui/portapack.rs`)**
- [ ] `pub fn render(f, area, m: &SdrMetrics)` — only called if `is_portapack`
- [ ] Shows battery gauge (voltage → percent via H2 discharge curve), active app name,
      GPS coordinates or "No Fix"
- [ ] Panel hidden entirely if `!is_portapack`
- [ ] Degrade gracefully: each field shows `N/A` if the control transfer fails

---

## Phase 11 — Polish & Production Readiness

**Goal:** The app feels finished. No rough edges, no crashes under adversarial use.

### UX steps

**11.1 — Startup UX**
- [ ] Show a non-TUI loading message while `Device::open` is in progress
      (opening a HackRF can take up to 500 ms)
- [ ] "No device found" shows a clean non-TUI error with a suggestion
      (`sudo` hint for udev rules, link to docs)
- [ ] Add `--version` flag (clap handles this automatically)

**11.2 — Terminal resize handling**
- [ ] Crossterm fires `Event::Resize(w, h)` automatically — ensure `EventStream`
      forwards it as `AppEvent::Resize(w, h)`
- [ ] In `App::run()`, on `AppEvent::Resize`, call `terminal.autoresize()` and
      re-trigger a draw; no restart required

**11.3 — Mouse support**
- [ ] Enable `EnableMouseCapture` (already done)
- [ ] Handle `Event::Mouse(MouseEvent { kind: ScrollUp, .. })` over the LNA gauge
      area → LNA +8 dB; `ScrollDown` → LNA −8 dB
- [ ] Handle `MouseEventKind::Down` on the device list panel rows → change `focused`
- [ ] Keep mouse optional: test that the app is fully usable keyboard-only

**11.4 — Themes**
- [ ] Define `Theme` struct in `src/ui/theme.rs`:
  ```rust
  pub struct Theme {
      pub border: Color, pub title: Color,
      pub gauge_lna: Color, pub gauge_vga: Color, pub gauge_sr: Color,
      pub sparkline: Color, pub spectrum_line: Color, pub peak_hold: Color,
      pub status_rx: Color, pub status_idle: Color,
      pub text: Color, pub dim: Color,
  }
  ```
- [ ] Implement `Theme::default()`, `Theme::gruvbox()`, `Theme::nord()`, `Theme::light()`
- [ ] Pass `&Theme` to every widget render function; remove hard-coded colors
- [ ] `t` key cycles through available themes; selected theme saved to config on exit

**11.5 — Panic safety**
- [ ] In `main()`, register `std::panic::set_hook` that calls
      `disable_raw_mode()` + `execute!(stdout, LeaveAlternateScreen)` before printing
      the panic message; without this a panic leaves the terminal in raw mode

**11.6 — Audit `unwrap()` calls**
- [ ] `grep -rn 'unwrap()' src/` — replace each with `?`, `expect("reason")`,
      or explicit error handling
- [ ] Exception: `lock().unwrap()` on `Mutex` is acceptable (poison = programming error)

**11.7 — `--no-color` flag**
- [ ] Add `#[arg(long)] no_color: bool` to `Args`
- [ ] If set (or if `NO_COLOR` env var is present), use `Theme::no_color()` which
      returns `Color::Reset` for everything

**11.8 — Performance**
- [ ] `cargo flamegraph -- --serial <sn>` while running at 20 Msps for 30 seconds
- [ ] Confirm: render loop ≥ 25 fps, FFT frame rate ≥ 15 fps, CPU < 30% on a
      modern i5/Ryzen 5
- [ ] Confirm: no `Mutex` lock held across an `await` point (would starve the async executor)
- [ ] Steady-state RSS < 50 MB (`/proc/<pid>/status | grep VmRSS`)

**11.9 — Integration test harness**
- [ ] Create `tests/mock_hackrf/` with a shared library (`libhackrf_mock.so`) that
      implements the `hackrf_*` symbols using in-process memory
- [ ] Write at least three integration tests:
  1. Device opens, gains can be set, metrics update
  2. RX streaming starts and stops cleanly
  3. Device disconnect is detected within one polling interval
- [ ] Wire into `cargo test` via `LD_PRELOAD` in a custom test runner script

---

## Phase 12 — Distribution & Community

**Goal:** Other people can install and use sdrtop without building from source.

### Packaging steps

**12.1 — AUR packages**
- [ ] Write `PKGBUILD` for `sdrtop-git` (builds from latest main)
- [ ] Write `PKGBUILD` for `sdrtop` (tracks tagged releases)
- [ ] Submit both to AUR; verify `makepkg -si` works on a clean Arch install

**12.2 — GitHub Actions CI**
- [ ] `.github/workflows/ci.yml`:
  - triggers: push to `main`, any PR
  - matrix: `ubuntu-latest`, `macos-latest`
  - steps: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`
- [ ] `.github/workflows/release.yml`:
  - triggers: `v*` tag push
  - matrix: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
             `x86_64-apple-darwin`, `aarch64-apple-darwin`
  - produces: stripped release binaries, uploads as GitHub Release assets

**12.3 — Nix flake**
- [ ] Write `flake.nix` with `packages.default = pkgs.rustPlatform.buildRustPackage`
- [ ] Test: `nix build` on NixOS and nix-on-Ubuntu

**12.4 — Homebrew formula**
- [ ] Write `Formula/sdrtop.rb` after macOS builds are verified
- [ ] Submit to homebrew-core or host in a personal tap

**12.5 — Documentation**
- [ ] `README.md`: installation section (AUR / nix / cargo / binary),
      one-paragraph feature summary, screenshot of the running TUI,
      full keybinding reference table
- [ ] `CONTRIBUTING.md`: dev environment setup, how to run the mock-device
      test harness, PR process, code style notes
- [ ] Man page: `clap`'s `generate_to` creates `sdrtop.1` at build time;
      install it via `Makefile` or the AUR PKGBUILD

---

## Key Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| `libhackrf` FFI struct differs across versions | crash / wrong data | check `hackrf_library_version()` at startup; bail if < minimum known-good |
| FFT worker can't keep up at 20 Msps | stale spectrum | bounded drop channel; `FftFrame.stale` flag; log overrun count |
| Terminal lacks Braille / truecolor | broken display | `ColorDepth::detect()` at startup; ASCII bar fallback for spectrum |
| USB disconnect mid-session | crash or hang | polling task catches error, marks device offline, recovers on reconnect |
| `main.rs` grows again | development friction | enforce: no file over 200 lines; `cargo clippy` as CI gate |
| Mutex poisoning under panic | terminal left in raw mode | `std::panic::set_hook` restores terminal unconditionally (Phase 11.5) |
