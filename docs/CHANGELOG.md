# Changelog

← [Home](Home.md)

Chronological record of shipped milestones and improvements.  
Full details are in the linked phase logs and improvement files.

---

## 2026-05-29

### IMP-003 — Spectrum & Waterfall UI Fixes
→ [details](improvements/imp-003-spectrum-waterfall-ui-fixes.md)

- **Spectrum border:** replaced three partial-border blocks with a single outer `Block::ALL`; eliminated double `╭` corner and unclosed bottom-right edge
- **Freq labels:** changed from fixed 12-char padding to `canvas_area.width`-proportional spacing; labels now land at 0 / 25 / 50 / 75 / 100% of the actual canvas width
- **Axis alignment:** waterfall canvas offset by 6 chars to match spectrum's dB-label column; same frequency now hits the same terminal column in both panels
- **Spectrum paint:** replaced filled Braille bars (per-bin vertical line from DB_MIN) with a gradient polyline (adjacent bin tops connected); cleaner outline without the dense fill
- **Waterfall legend:** left 6-char column now shows a dBFS color scale (`█` strip with themed gradient + labels at +0 / −60 / −120 dBFS)

---

## 2026-05-28

### Phase 12 — UI/UX Polish & Theme System 🔧 In progress
→ [log](<phases/Phase 12 - UI UX Polish Theme System - Log.md>)

- Theme system: `Theme` struct with 6 built-in palettes (`sdr`, `nord`, `dracula`, `gruvbox`, `catppuccin`, `solarized`); `[theme]` TOML section with per-field `#rrggbb` overrides; `--theme` CLI flag
- All 13 panels migrated to `BorderType::Rounded`; three border tiers by panel role; stale / observer states get dedicated colors
- Spectrum: per-bin gradient computed outside Canvas closure (ratatui 0.26 `'static` constraint workaround)
- `HeaderPanel` redesigned as stateless; reads live from `SdrMetrics`
- `FooterPanel` redesigned with four context modes: observer / text-input / panel-focused / normal
- Panel focus system: 7 panels register focus keys (`e o h c m i g`); `LayoutEngine` tracks `focused_panel`; `Esc` exits

### IMP-002 — Observer Mode ✅ (alpha)
→ [plan](improvements/imp-002-observer-mode.md) · [log](improvements/imp-002-observer-mode-log.md)

- When another app holds the HackRF exclusively, sdrtop enters observer mode instead of crashing
- Reads device identity, USB stats, and owner process info from sysfs + `/proc` — no libhackrf needed
- New `ObserverPanel`; dedicated `observer` preset; all hardware controls silently gated

### IMP-001 — Interactive Sample Rate Control ✅
→ [details](improvements/imp-001-sample-rate-control.md)

- `[S]` key opens a sample-rate input mode in the footer
- Validates 2–20 MHz range; calls `device.set_sample_rate()` on confirm; rejects invalid input with log message

---

## Earlier phases

Exact dates not recorded for phases 1–11. See individual phase logs for details.

### Phase 11 — HackRF Deep Diagnostics ✅
→ [steps](<phases/Phase 11 - HackRF Deep Diagnostics - Steps.md>) · [log](<phases/Phase 11 - HackRF Deep Diagnostics - Log.md>)

New data: board revision, USB API version, CPLD checksum, SNR, channel power, occupied BW, IQ amplitude histogram.  
New panels: `RfChainPanel`, `SignalMetricsPanel`, `IqHistogramPanel`. New preset: `lab` (key `6`).

### Phase 10 — Configuration & Persistence ✅
→ [steps](<phases/Phase 10 - Configuration & Persistence - Steps.md>) · [log](<phases/Phase 10 - Configuration & Persistence - Log.md>)

Radio settings and display state persist across restarts via `~/.config/sdrtop/config.toml`.  
Atomic save (write `.tmp` → rename); CLI args override file; missing/corrupt file silently defaults.

### Phase 9 — Waterfall Display ✅
→ [steps](<phases/Phase 9 - Waterfall Display - Steps.md>) · [log](<phases/Phase 9 - Waterfall Display - Log.md>)

Scrolling 2D spectrum history as background-colored terminal cells.  
`ColorDepth` detection; truecolor piecewise gradient (6 stops, dark blue → red); 16-color fallback.  
New presets: `waterfall` (key `4`), `spectrum_waterfall` (key `5`).

### Phase 8 — FFT Spectrum Analyzer ✅
→ [8a steps](<phases/Phase 8a - FFT Pipeline - Steps.md>) · [8b steps](<phases/Phase 8b - Spectrum Display - Steps.md>) · [log](<phases/Phase 8 - FFT Spectrum Analyzer - Log.md>)

Live Braille canvas spectrum with peak hold, noise floor, dBFS and frequency axes.  
`FftWorker`: Hann window → rustfft → dBFS → fftshift → EMA smoothing → peak-hold decay → noise floor.  
`RxContext` pattern for safe FFI callback; lock released before IQ buffer allocation.

### Phase 7 — Hardware Health Panels ✅
→ [steps](<phases/Phase 7 - Hardware Health Panels - Steps.md>) · [log](<phases/Phase 7 - Hardware Health Panels - Log.md>)

New panels: `HardwareHealthPanel` (drops, ADC saturation, jitter), `IqDiagnosticsPanel`, `SystemResourcesPanel`.  
Accumulator pattern: integer sums in `rx_callback`, float metrics computed in polling task.

### Phase 6 — Dashboard Engine ✅
→ [steps](<phases/Phase 6 - Dashboard Engine - Steps.md>) · [log](<phases/Phase 6 - Dashboard Engine - Log.md>)

`Panel` trait + `PanelRegistry` + `LayoutEngine` with top/body/bottom zones and left/center/right columns.  
`LayoutConfig` is serde-deserializable — preset loading in Phase 10 required no engine changes.

### Phase 5 — Interactive Controls ✅
→ [steps](<phases/Phase 5 - Interactive Controls - Steps.md>) · [log](<phases/Phase 5 - Interactive Controls - Log.md>)

Full keyboard control: LNA, VGA, AMP, frequency input, reset. `InputMode` enum drives two-level event loop.  
Help overlay rendered last in `draw()` to layer on top of all panels.

### Phase 4 — Architecture Refactor ✅
→ [steps](<phases/Phase 4 - Architecture Refactor - Steps.md>) · [log](<phases/Phase 4 - Architecture Refactor - Log.md>)

`main.rs` reduced from ~670 lines to 43 — pure entry point.  
Six focused modules established; stub files added for all future phases.

### Phase 3 — TUI Dashboard ✅
→ [steps](<phases/Phase 3 - TUI Dashboard - Steps.md>) · [log](<phases/Phase 3 - TUI Dashboard - Log.md>)

Live ratatui dashboard: header, telemetry panel, gain gauges, 64-point throughput sparkline, log panel, footer.

### Phase 2 — Telemetry Polling & USB Throughput ✅
→ [steps](<phases/Phase 2 - Telemetry Polling - Steps.md>) · [log](<phases/Phase 2 - Telemetry Polling - Log.md>)

`Arc<Mutex<SdrMetrics>>` shared between UI thread and tokio background task.  
`rx_enabled` / `hw_streaming` split fixed a dual-state bug in the original design.

### Phase 1 — Device Discovery ✅
→ [steps](<phases/Phase 1 - Device Discovery - Steps.md>) · [log](<phases/Phase 1 - Device Discovery - Log.md>)

Hand-crafted `#[repr(C)]` FFI layer; critical `HackrfDeviceList` struct layout fixed.  
Safe `Device` wrapper with `Drop` ensuring `hackrf_exit()` on all exit paths.
