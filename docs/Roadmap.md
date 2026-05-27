# sdrtop — Roadmap to btop-level Quality

← [Home](Home.md)

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

| Phase                                                                  | Status     |
| ---------------------------------------------------------------------- | ---------- |
| 1 — Device discovery & basic info                                      | ✅ Done     |
| 2 — Telemetry polling & USB throughput                                 | ✅ Done     |
| 3 — TUI dashboard (gauges, sparkline, log, shortcuts)                  | ✅ Done     |
| 4 — Architecture refactor (modular layout)                             | ✅ Done     |
| 5 — Interactive controls                                               | ✅ Done     |
| 6 — Dashboard engine (panel system, presets, layout config)            | ✅ Done     |
| 7 — Hardware health panels (drop rate, ADC saturation, IQ diagnostics) | ✅ Done     |
| 8 — FFT spectrum analyzer                                              | 🔲 Next    |
| 9 — Waterfall display                                                  | 🔲 Planned |
| 10 — Configuration & persistence                                       | 🔲 Planned |
| 11 — Multi-device support                                              | 🔲 Planned |
| 12 — PortaPack / Mayhem integration                                    | 🔲 Planned |
| 13 — Polish & production readiness                                     | 🔲 Planned |
| 14 — Distribution & community                                          | 🔲 Planned |

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

## Phase 1 — Device Discovery & Basic Info ✅ Done

**Goal:** Open a HackRF device via a hand-crafted libhackrf FFI layer and read
its identity: board name, firmware version, and serial number.

- Step-by-step execution guide: [Phase 1 - Device Discovery - Steps](phases/Phase%201%20-%20Device%20Discovery%20-%20Steps.md)
- Implementation log (what was done, decisions made): [Phase 1 - Device Discovery - Log](phases/Phase%201%20-%20Device%20Discovery%20-%20Log.md)

### Key outcomes

- Custom `#[repr(C)]` FFI layer bypassing the broken `hackrf` 0.0.1 crate
- Critical `HackrfDeviceList` struct layout fixed (missing fields, wrong types)
- Safe `Device` wrapper with `Drop` ensuring clean `hackrf_exit()` on all exit paths

---

## Phase 2 — Telemetry Polling & USB Throughput ✅ Done

**Goal:** Start RX streaming and measure live USB throughput via a tokio background
task. Shared state updated every 200 ms behind `Arc<Mutex<SdrMetrics>>`.

- Step-by-step execution guide: [Phase 2 - Telemetry Polling - Steps](phases/Phase%202%20-%20Telemetry%20Polling%20-%20Steps.md)
- Implementation log (what was done, decisions made): [Phase 2 - Telemetry Polling - Log](phases/Phase%202%20-%20Telemetry%20Polling%20-%20Log.md)

### Key outcomes

- `Arc<Mutex<SdrMetrics>>` shared between UI thread and background polling task
- Critical bug fixed: single `is_streaming` split into `rx_enabled` (desired, UI only) and `hw_streaming` (actual, polling only)
- `rx_callback` accumulates bytes; polling task computes throughput every 200 ms using integer arithmetic

---

## Phase 3 — TUI Dashboard ✅ Done

**Goal:** Live ratatui dashboard with telemetry panel, gain gauges, USB throughput
sparkline, log panel, and keyboard shortcuts.

- Step-by-step execution guide: [Phase 3 - TUI Dashboard - Steps](phases/Phase%203%20-%20TUI%20Dashboard%20-%20Steps.md)
- Implementation log (what was done, decisions made): [Phase 3 - TUI Dashboard - Log](phases/Phase%203%20-%20TUI%20Dashboard%20-%20Log.md)

### Key outcomes

- Layout: header / body (telemetry left + gauges right) / log / footer
- Added: serial number in header, sample rate gauge, 64-point throughput sparkline, 7-row log panel, `r` reset key
- Footer shows only implemented keys — misleading Phase 5 shortcuts (`F`, `S`, `L`, `V`, `A`) removed

---

## Phase 4 — Architecture Refactor ✅ Done

**Goal:** `main.rs` becomes an entry point only. Split into focused modules before
adding more features. Every future phase has a clean home; no file exceeds ~200 lines.

- Step-by-step execution guide: [Phase 4 - Architecture Refactor - Steps](phases/Phase%204%20-%20Architecture%20Refactor%20-%20Steps.md)
- Implementation log (what was done, decisions made): [Phase 4 - Architecture Refactor - Log](phases/Phase%204%20-%20Architecture%20Refactor%20-%20Log.md)

### Key outcomes

- `main.rs` reduced from ~670 lines to 43 — pure entry point, no logic
- Six focused modules: `state`, `event`, `app`, `hardware/{ffi,device}`, `ui/{layout,header,telemetry,gains,log,footer}`
- `rx_callback` and `Device` wrapper isolated in `hardware/device.rs`; UI modules have no FFI dependencies
- Stub files added for all future phases so every new feature has a clear home from day one

### Final module layout

```
src/
  main.rs               43 lines — terminal setup/teardown + App::new()?.run()
  app.rs                App struct + new() + run()
  event.rs              AppEvent enum, EventStream (mpsc + thread)
  state.rs              SdrMetrics, constants
  config.rs             stub — Phase 10
  hardware/
    mod.rs              pub use device::Device
    ffi.rs              #[repr(C)] structs + pub extern "C" declarations
    device.rs           Device wrapper + rx_callback
    buffer.rs           stub — Phase 8
  ui/
    mod.rs              pub fn draw(frame, state, ...)
    layout.rs           Chunks struct + build(size)
    header.rs           render(f, area, board_name, fw, serial)
    telemetry.rs        render(f, area, m, board_name, serial)
    gains.rs            render(f, area, m) — gauges + sparkline
    log.rs              render(f, area, m)
    footer.rs           render(f, area)
    overlay.rs          stub — Phase 5
    sparkline.rs        stub — Phase 5+
    spectrum.rs         stub — Phase 8
    waterfall.rs        stub — Phase 9
```

---

## Phase 5 — Interactive Controls ✅ Done

**Goal:** Every parameter visible in the UI can be changed live from the keyboard.
Hardware is called immediately; the display reflects the new value within one render frame.

- Step-by-step execution guide: [Phase 5 - Interactive Controls - Steps](phases/Phase%205%20-%20Interactive%20Controls%20-%20Steps.md)
- Implementation log (what was done, decisions made): [Phase 5 - Interactive Controls - Log](phases/Phase%205%20-%20Interactive%20Controls%20-%20Log.md)

### Key outcomes

- Full keyboard control of all radio parameters: LNA (±8 dB, 0–40), VGA (±2 dB, 0–62), AMP toggle, frequency input, reset — hardware called immediately on every keypress, display stays at last confirmed value on error
- `InputMode` enum added to `SdrMetrics` drives a two-level event loop (`InputMode` → `KeyCode`) and context-aware footer rendering
- Frequency input mode with three-outcome handling: parse failure stays in input mode, hardware failure stays in input mode so the user can retry, success returns to Normal and logs confirmation
- Help overlay (`overlay.rs`) rendered last in `draw()` so it appears on top of all panels; `show_help` lives on `App` (not `SdrMetrics`) as UI-only state
- Reset key now calls all five hardware setters before `reset_to_defaults()`; each error is logged individually, reset proceeds regardless

---

## Phase 6 — Dashboard Engine ✅ Done

**Goal:** Replace the fixed TUI layout with a modular panel system where every display
element is a named, self-contained unit. The user controls which panels are shown and
where, via preset switching and a config file.

- Step-by-step execution guide: [Phase 6 - Dashboard Engine - Steps](phases/Phase%206%20-%20Dashboard%20Engine%20-%20Steps.md)
- Implementation log (what was done, decisions made): [Phase 6 - Dashboard Engine - Log](phases/Phase%206%20-%20Dashboard%20Engine%20-%20Log.md)

### Key outcomes

- `Panel` trait (`name`, `min_size`, `render`) — every current and future display element implements it; adding a panel requires only the impl + one `registry.register()` call
- `PanelRegistry` wraps `HashMap<&'static str, Box<dyn Panel>>`; panels not in the active preset are registered but never rendered, costing no CPU
- `LayoutEngine` splits the terminal into top / body / bottom zones, then the body into left / center / right columns; column width is driven by `width_pct` of the first panel in each column (summing was the bug in the original plan)
- `LayoutConfig` is serde-deserializable — presets will be loadable from `~/.config/sdrtop/config.toml` in Phase 10 without further changes to the engine
- `show_help` and overlay rendering stay on `App`, outside the panel system — the engine knows nothing about help state
- `board_name / fw_version / serial` removed from `App` struct; the values live inside `HeaderPanel` and `TelemetryPanel` where they are actually used

---

## Phase 7 — Hardware Health Panels ✅ Done

**Goal:** Make sample drops, ADC saturation, IQ quality, and system resource usage
visible in real time — the metrics that turn sdrtop from an SDR frontend into a
genuine resource monitor. All three are new `Panel` plugins.

- Step-by-step execution guide: [Phase 7 - Hardware Health Panels - Steps](phases/Phase%207%20-%20Hardware%20Health%20Panels%20-%20Steps.md)
- Implementation log (what was done, decisions made): [Phase 7 - Hardware Health Panels - Log](phases/Phase%207%20-%20Hardware%20Health%20Panels%20-%20Log.md)

### Key outcomes

- Three new panels registered in the Phase 6 engine: `HardwareHealthPanel`, `IqDiagnosticsPanel`, `SystemResourcesPanel`
- `monitoring` preset: two-column layout with health panels left, telemetry+resources right; `2` key switches to it, `1` returns to minimal
- Accumulator pattern: `rx_callback` (C thread) accumulates integer sums only; polling task snapshots+resets+computes float metrics atomically in one lock acquisition
- System resource task spawned independently from hardware task: reads `/proc/self/stat` (parsed with `rsplit_once(')')` to handle process names with spaces) and `/proc/self/status` every second
- Clippy `-D warnings` clean: manual checked-division patterns converted to `checked_div`

---

## Phase 8 — FFT Spectrum Analyzer 🔲 Next

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
  - draw a filled bar for each bin using Braille dots
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
- [ ] Add `w` key to cycle window function: Hann → Hamming → Blackman → Hann

**6.7 — Benchmark**
- [ ] Run with a real HackRF at 20 Msps; verify FFT frame rate ≥ 10 fps
- [ ] On Raspberry Pi 4 (if available): target ≥ 5 fps at 2048-point FFT
- [ ] `cargo build --release` — profile build must pass clean

---

## Phase 9 — Waterfall Display 🔲 Planned

**Goal:** A scrolling 2D spectrum history below the spectrum plot.

### Color palette

| Terminal capability | Palette used |
|---|---|
| Truecolor (`COLORTERM=truecolor`) | 24-bit RGB gradient: `#000080` → `#00ff00` → `#ffff00` → `#ff0000` |
| 256-color | pre-computed 32-entry lookup into xterm-256 palette |
| 16-color fallback | 4 levels: black, dark blue, cyan, white |

### Steps

**7.1 — WaterfallBuffer**
- [ ] In `state.rs`, add `WaterfallBuffer` struct with `push`, `paused`, `max_rows`
- [ ] Add `waterfall: WaterfallBuffer` to `SdrMetrics`
- [ ] Update FFT frame consumer in `app.rs`

**7.2 — Color palette (`src/palette.rs`)**
- [ ] `ColorDepth` enum + `detect()` (reads `COLORTERM` env var)
- [ ] `magnitude_to_color(db, db_min, db_max, depth) -> Color`

**7.3 — WaterfallWidget (`src/ui/waterfall.rs`)**
- [ ] `pub fn render(f, area, buf, db_min, db_max, depth)`
- [ ] Canvas with solid colored blocks (1 col × 1 row per cell)

**7.4 — Layout integration**
- [ ] `show_waterfall: bool` and `waterfall_height: u16` in `SdrMetrics`
- [ ] Conditional spectrum/waterfall split in `ui/layout.rs`

**7.5 — Keyboard controls**
- [ ] `w` cycles display mode: Spectrum → Both → Waterfall only
- [ ] `p` toggles `waterfall.paused`

**7.6 — Validation**
- [ ] At 80×24, 2048-point FFT, `Both` mode: render stays ≥ 10 fps
- [ ] Palette degrades correctly with `COLORTERM` unset

---

## Phase 10 — Configuration & Persistence 🔲 Planned

**Goal:** Settings survive restarts.

### Config schema (`~/.config/sdrtop/config.toml`)

```toml
[device]
serial = ""

[radio]
frequency_hz = 2400000000
sample_rate  = 10000000.0
lna_gain     = 16
vga_gain     = 20
amp_enabled  = false
fft_size     = 2048
fft_window   = "hann"

[display]
spectrum_height  = 14
waterfall_rows   = 20
spectrum_db_min  = -120
spectrum_db_max  = 0
theme            = "default"
show_waterfall   = true
```

### Steps

**8.1** — Add `serde`, `toml`, `clap` to `Cargo.toml`  
**8.2** — Define `Config` struct (`src/config.rs`) with nested sections  
**8.3** — `Config::load_or_default(path)` — missing file returns default, parse error logs warning  
**8.4** — `Config::save(&self, path)` — atomic write via `.tmp` + rename  
**8.5** — CLI args via `clap` (`--config`, `--frequency`, `--lna`, `--vga`, `--serial`)  
**8.6** — Apply config to initial `SdrMetrics` in `App::new()`  
**8.7** — Save on `q` exit; best-effort save via `std::panic::set_hook`

---

## Phase 11 — Multi-Device Support 🔲 Planned

**Goal:** Multiple HackRF devices monitored simultaneously; `Tab` switches focus.

### Steps

**9.1** — Introduce `DeviceHandle` struct; refactor `App` to hold `Vec<DeviceHandle>`  
**9.2** — Open all connected devices at startup; spawn one polling task + FFT worker per device  
**9.3** — Device list panel (`src/ui/device_list.rs`); `d` key toggles; `Tab` changes focus  
**9.4** — Disconnect detection; mark device offline, stop FFT worker  
**9.5** — Reconnect detection via 2-second watcher task

---

## Phase 12 — PortaPack / Mayhem Integration 🔲 Planned

**Goal:** Show Mayhem-specific telemetry when a PortaPack is connected.

### Known telemetry (USB vendor control transfers)

| Data | bRequest |
|---|---|
| Battery voltage (mV) | 0x10 |
| Active application | 0x11 |
| GPS fix + coordinates | 0x12 |

### Steps

**10.1** — USB product string detection (`"PortaPack"` → `device.is_portapack = true`)  
**10.2** — `Device::vendor_read(request, buf)` helper  
**10.3** — PortaPack telemetry polling in the background task  
**10.4** — PortaPack panel (`src/ui/portapack.rs`), hidden if `!is_portapack`

---

## Phase 13 — Polish & Production Readiness 🔲 Planned

**Steps**

**11.1** — Startup UX: loading message, clean "no device" error  
**11.2** — Terminal resize: forward `Event::Resize` as `AppEvent::Resize`  
**11.3** — Mouse support: scroll over gauges, click device list  
**11.4** — Themes: `default`, `gruvbox`, `nord`, `light`; `t` key cycles  
**11.5** — Panic hook: restore terminal unconditionally before printing panic  
**11.6** — Audit `unwrap()` calls; replace with `?` or `expect("reason")`  
**11.7** — `--no-color` flag + `NO_COLOR` env var  
**11.8** — Performance: flamegraph, ≥25 fps render, <30% CPU, <50 MB RSS  
**11.9** — Integration test harness with `libhackrf_mock.so`

---

## Phase 14 — Distribution & Community 🔲 Planned

**Steps**

**12.1** — AUR packages (`sdrtop-git` and `sdrtop`)  
**12.2** — GitHub Actions CI (lint + test) and release matrix (4 targets)  
**12.3** — Nix flake  
**12.4** — Homebrew formula  
**12.5** — `README.md`, `CONTRIBUTING.md`, man page via `clap`

---

## Key Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| `libhackrf` FFI struct differs across versions | crash / wrong data | check `hackrf_library_version()` at startup |
| FFT worker can't keep up at 20 Msps | stale spectrum | bounded drop channel; `FftFrame.stale` flag |
| Terminal lacks Braille / truecolor | broken display | `ColorDepth::detect()` at startup; ASCII fallback |
| USB disconnect mid-session | crash or hang | polling task catches error, recovers on reconnect |
| `main.rs` grows again | development friction | no file over 200 lines; clippy as CI gate |
| Mutex poisoning under panic | terminal in raw mode | `std::panic::set_hook` restores terminal (Phase 13.5) |
