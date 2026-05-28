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
| 8 — FFT spectrum analyzer                                              | ✅ Done     |
| 9 — Waterfall display                                                  | ✅ Done     |
| 10 — Configuration & persistence                                       | ✅ Done     |
| 11 — HackRF deep diagnostics                                           | ✅ Done     |
| 12 — PortaPack / Mayhem integration                                    | 🔲 Next    |
| 13 — Multi-device support                                              | 🔲 Planned |
| 14 — Polish & production readiness                                     | 🔲 Planned |
| 15 — Distribution & community                                          | 🔲 Planned |

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

## Phase 8 — FFT Spectrum Analyzer ✅ Done

**Goal:** A live spectrum display on a Braille canvas — green bars, yellow peak-hold,
gray noise floor, labeled frequency and dBFS axes. The feature that makes `sdrtop`
genuinely useful for RF work.

- Phase 8a step-by-step guide: [Phase 8a - FFT Pipeline - Steps](phases/Phase%208a%20-%20FFT%20Pipeline%20-%20Steps.md)
- Phase 8b step-by-step guide: [Phase 8b - Spectrum Display - Steps](phases/Phase%208b%20-%20Spectrum%20Display%20-%20Steps.md)
- Implementation log (what was done, decisions made): [Phase 8 - FFT Spectrum Analyzer - Log](phases/Phase%208%20-%20FFT%20Spectrum%20Analyzer%20-%20Log.md)

### Key outcomes

- `RxContext` replaces bare `*const Mutex` pointer in `rx_callback` — bundles metrics Arc and crossbeam channel sender; lock released before IQ buffer allocation
- `FftWorker` on `std::thread` (not tokio): accumulate → Hann window → rustfft → dBFS (normalized by fft_size) → fftshift → EMA (α=0.2) → peak-hold (0.5 dB/frame decay) → noise floor (mean of bottom 10% bins)
- `SpectrumPanel` Canvas layout: left 6 cols dBFS labels, right canvas with `CanvasLine` bars (green) + `Points` peak-hold (yellow) + horizontal noise floor (gray), 1 row frequency axis below
- `spectrum` preset (full-width Body); `3` key switches to it; `p` cycles all three presets
- Stale detection: `frame.timestamp.elapsed() > 500 ms` → `[STALE]` in panel title

---

## Phase 9 — Waterfall Display ✅ Done

**Goal:** A scrolling 2D spectrum history rendered as rows of background-colored
terminal cells — color encodes signal strength. Two new presets: `waterfall`
(standalone) and `spectrum_waterfall` (spectrum above, waterfall below).

- Step-by-step execution guide: [Phase 9 - Waterfall Display - Steps](phases/Phase%209%20-%20Waterfall%20Display%20-%20Steps.md)
- Implementation log (what was done, decisions made): [Phase 9 - Waterfall Display - Log](phases/Phase%209%20-%20Waterfall%20Display%20-%20Log.md)

### Key outcomes

- `WaterfallBuffer` (VecDeque, newest row at front, max 64 rows) pushed by FftWorker in the same lock as `FftFrame` — spectrum and waterfall always in sync
- `src/palette.rs`: `ColorDepth` detected once at startup; truecolor piecewise gradient (6 stops, dark blue → red), 16-step xterm-256 lookup, 4-level 16-color fallback
- `WaterfallPanel` renders background-colored space characters (`Span::styled(" ", Style::bg(color))`) — no Canvas widget needed
- `spectrum_waterfall` preset: two `Position::Body` panels split equally by the existing `render_column` — no engine changes needed
- `4` → waterfall, `5` → spectrum+waterfall, `w` → pause/resume; `p` cycles all five presets

---

## Phase 10 — Configuration & Persistence ✅ Done

**Goal:** Radio settings and display state survive restarts. Read from
`~/.config/sdrtop/config.toml` at startup; written back on clean exit (`q`).
CLI flags override config file values.

- Step-by-step execution guide: [Phase 10 - Configuration & Persistence - Steps](phases/Phase%2010%20-%20Configuration%20%26%20Persistence%20-%20Steps.md)
- Implementation log (what was done, decisions made): [Phase 10 - Configuration & Persistence - Log](phases/Phase%2010%20-%20Configuration%20%26%20Persistence%20-%20Log.md)

### Config schema

```toml
[radio]
frequency_hz = 2400000000
sample_rate  = 10000000.0
lna_gain     = 16
vga_gain     = 20
amp_enabled  = false

[display]
active_preset      = "minimal"
waterfall_max_rows = 64
```

### Key outcomes

- `AppConfig` with `RadioConfig` + `DisplayConfig`; partial TOML handled via per-field `#[serde(default = "fn")]` annotations (not just section-level `#[serde(default)]`)
- `load_or_default`: missing file → silent default; parse error → warning on stderr + default; app always starts
- Atomic save: write `.tmp` then `rename` — previous config intact if crash mid-write
- Merge order: `Default` → config file → CLI args (`--frequency`, `--lna`, `--vga`)
- `save_config()` extracts values from `SdrMetrics` while holding the mutex, drops the guard, then writes to disk — file I/O never blocks the render loop
- Hardware settings applied at startup (best-effort); startup errors logged in-app, streaming doesn't begin until Space is pressed
- `clap 4` derive: `--config`, `--frequency`, `--lna`, `--vga`; `$HOME` env var for default path, no new crate

---

## Phase 11 — HackRF Deep Diagnostics ✅ Done

**Goal:** Surface every hardware metric the HackRF exposes but is not yet shown, and
add DSP-derived RF quality metrics. After this phase sdrtop gives a complete lab-grade
picture of the hardware and signal chain.

- Step-by-step execution guide: [Phase 11 - HackRF Deep Diagnostics - Steps](phases/Phase%2011%20-%20HackRF%20Deep%20Diagnostics%20-%20Steps.md)

### New data sources

| Source | Data | How |
|---|---|---|
| `hackrf_board_rev_read` | Board revision (r6/r7/r8/r9/r10) | libhackrf at startup |
| `hackrf_usb_api_version_read` | USB API protocol version | libhackrf at startup |
| `hackrf_cpld_checksum` | CPLD CRC integrity | libhackrf at startup |
| Computed from state | Baseband filter BW, total gain (LNA+VGA+AMP) | pure function in panel |
| Extended FftWorker | SNR, channel power (dBFS), occupied BW (99%) | per FFT frame |
| Extended rx_callback accumulator | IQ amplitude histogram (32 bins) | per sample batch |

### New panels

- **`RfChainPanel`** — complete gain chain + board revision + CPLD status + BB filter BW
- **`SignalMetricsPanel`** — SNR, channel power, 99% occupied BW, noise floor (all live)
- **`IqHistogramPanel`** — amplitude distribution bar chart; shows dynamic range utilisation and clipping

### New preset: `lab`

Six-panel layout: `rf_chain` + `iq_diagnostics` (left) | `signal_metrics` + `iq_histogram` (right) | `hardware_health` + `system_resources` (right). Key `6`.

### Steps

**11.1** — New `SdrMetrics` fields + FftFrame extensions + IQ histogram accumulator fields + 5 unit tests  
**11.2** — New FFI declarations + `Device` methods (`board_rev`, `usb_api_version`, `cpld_checksum`) + unit tests  
**11.3** — `App::new()` reads board rev / USB API version / CPLD checksum at startup  
**11.4** — Extended `rx_callback` accumulator: 32-bin IQ amplitude histogram  
**11.5** — Extended `FftWorker`: SNR, channel power, occupied BW computed per frame  
**11.6** — `RfChainPanel` + `SignalMetricsPanel` + `IqHistogramPanel`  
**11.7** — Register panels, `lab` preset, key `6`, overlay update

---

## Phase 12 — PortaPack / Mayhem Integration 🔲 Next

**Goal:** Detect a PortaPack Mayhem device and display live telemetry (firmware version,
platform model, RTC clock) in a dedicated panel. Auto-detection on startup; reconnect on
unplug/replug.

- Step-by-step execution guide: [Phase 12 - PortaPack Mayhem Integration - Steps](phases/Phase%2012%20-%20PortaPack%20Mayhem%20Integration%20-%20Steps.md)

### Protocol

PortaPack Mayhem exposes a USB CDC/ACM serial interface (`/dev/ttyACM*` on Linux)
running a text shell, independent of the HackRF RF interface already held by libhackrf.

```
host → device:   info\r
device → host:   Mayhem: 2.1.0\r\n
                 Platform: PortaPack H2\r\n
                 ok\r\n
```

Detection: open `/dev/ttyACM*`, send `info\r`, check response for `"Mayhem"`.

### Steps

**12.1** — `serialport = "4"` dep + `PortaPackState` in `state.rs` + 3 unit tests  
**12.2** — `src/portapack.rs`: `find_portapack()`, `send_command()`, `PortaPackWorker` + 4 unit tests  
**12.3** — `App` integration: spawn worker thread, add `'7'` preset key  
**12.4** — `src/ui/portapack_panel.rs`: `PortaPackPanel` (connected/disconnected state)  
**12.5** — Register panel, add `portapack` preset to `LayoutConfig`, update overlay

---

## Phase 13 — Multi-Device Support 🔲 Planned

**Goal:** Multiple HackRF devices monitored simultaneously; `Tab` switches focus.

### Steps

**13.1** — Introduce `DeviceHandle` struct; refactor `App` to hold `Vec<DeviceHandle>`  
**13.2** — Open all connected devices at startup; spawn one polling task + FFT worker per device  
**13.3** — Device list panel (`src/ui/device_list.rs`); `d` key toggles; `Tab` changes focus  
**13.4** — Disconnect detection; mark device offline, stop FFT worker  
**13.5** — Reconnect detection via 2-second watcher task

---

## Phase 14 — Polish & Production Readiness 🔲 Planned

**Steps**

**14.1** — Startup UX: loading message, clean "no device" error  
**14.2** — Terminal resize: forward `Event::Resize` as `AppEvent::Resize`  
**14.3** — Mouse support: scroll over gauges, click device list  
**14.4** — Themes: `default`, `gruvbox`, `nord`, `light`; `t` key cycles  
**14.5** — Panic hook: restore terminal unconditionally before printing panic  
**14.6** — Audit `unwrap()` calls; replace with `?` or `expect("reason")`  
**14.7** — `--no-color` flag + `NO_COLOR` env var  
**14.8** — Performance: flamegraph, ≥25 fps render, <30% CPU, <50 MB RSS  
**14.9** — Integration test harness with `libhackrf_mock.so`

---

## Phase 15 — Distribution & Community 🔲 Planned

**Steps**

**15.1** — AUR packages (`sdrtop-git` and `sdrtop`)  
**15.2** — GitHub Actions CI (lint + test) and release matrix (4 targets)  
**15.3** — Nix flake  
**15.4** — Homebrew formula  
**15.5** — `README.md`, `CONTRIBUTING.md`, man page via `clap`

---

## Key Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| `libhackrf` FFI struct differs across versions | crash / wrong data | check `hackrf_library_version()` at startup |
| FFT worker can't keep up at 20 Msps | stale spectrum | bounded drop channel; `FftFrame.stale` flag |
| Terminal lacks Braille / truecolor | broken display | `ColorDepth::detect()` at startup; ASCII fallback |
| USB disconnect mid-session | crash or hang | polling task catches error, recovers on reconnect |
| `main.rs` grows again | development friction | no file over 200 lines; clippy as CI gate |
| Mutex poisoning under panic | terminal in raw mode | `std::panic::set_hook` restores terminal (Phase 14.5) |
