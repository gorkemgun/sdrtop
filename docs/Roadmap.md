# sdrtop — Roadmap

← [Home](Home.md)

---

## Vision

`sdrtop` is a terminal-based SDR monitor in the spirit of `btop`: visually rich, fully interactive, and genuinely useful as a daily driver. The end state is a **universal SDR diagnostic tool** — one terminal app that works with any radio on your desk, shows everything the hardware is doing, and gets out of the way.

An RF engineer, ham, or security researcher should reach for `sdrtop` the way a sysadmin reaches for `btop`: as the first thing they open, not as one tool among many.

**Design principles:**
- Every metric shown must be hardware-sourced (not inferred from software)
- Adding a new SDR backend should not require changing the UI layer
- The TUI should be as information-dense as the terminal allows, without being cluttered

---

## Current status

11 of 23 phases complete. Phase 12 (UI/UX polish & theme system) is currently in progress. The core RF toolchain — FFT pipeline, spectrum, waterfall — is done and running on real hardware.

**Available hardware:** HackRF One · PortaPack H4M (Mayhem)

Phases 13–17 are fully achievable with current hardware.  
Phases 18–21 are blocked on physical devices — see [Supporting the project](../README.md#supporting-the-project).

| Phase | Title | Status |
|---|---|---|
| 1–11 | Core toolchain (device → FFT → waterfall → theme foundation) | ✅ Done |
| 12 | UI/UX polish & theme system | 🔧 **In progress** |
| 13 | Hardware abstraction layer (HAL) | 🔲 Next |
| 14 | PortaPack / Mayhem integration | 🔲 Planned |
| 15 | HackRF feature pass | 🔲 Planned |
| 16 | Polish & production readiness | 🔲 Planned |
| 17 | Distribution & community | 🔲 Planned |
| 18 | RTL-SDR support | ⏳ Needs hardware |
| 19 | Airspy / Airspy HF+ support | ⏳ Needs hardware |
| 20 | SoapySDR backend | ⏳ Needs hardware |
| 21 | Multi-device monitor | ⏳ Needs hardware |
| 22 | Advanced UI | 🔲 Planned |
| 23 | Ecosystem | 🔲 Planned |

Full phase history with per-phase logs: [Home](Home.md#phase-progress).

---

## Technology stack

| Concern | Choice | Notes |
|---|---|---|
| Language | Rust stable | |
| TUI | `ratatui 0.26+` | layout, widgets, Braille canvas |
| Hardware FFI | `libhackrf` via `pkg-config` | custom FFI — bypasses broken `hackrf` 0.0.1 crate |
| RTL-SDR FFI | `librtlsdr` via `pkg-config` | Phase 18 |
| Airspy FFI | `libairspy` + `libairspyhf` | Phase 19 |
| SoapySDR | SoapySDR C API | Phase 20; optional feature flag |
| Async runtime | `tokio` | background polling & FFT task |
| FFT | `rustfft 6` | pure-Rust, no C dependency |
| Config | `toml 0.8` + `serde 1` | `~/.config/sdrtop/config.toml` |
| CLI args | `clap 4` (derive) | |
| Channels | `crossbeam-channel 0.5` | lock-free IQ sample handoff |

---

## Phase 13 — Hardware Abstraction Layer (HAL) 🔧

**Why first:** Adding RTL-SDR, Airspy, or SoapySDR directly onto the current HackRF-coupled architecture would require rewriting the UI layer each time. The HAL defines a `SdrDevice` trait and refactors HackRF to implement it — every future backend slots in without touching the display code.

This phase is pure Rust refactoring. No new hardware required.

| Step | Description |
|---|---|
| 13.1 | Define `SdrDevice` trait in `src/device/mod.rs`: `open()`, `start_rx()`, `stop_rx()`, `set_frequency()`, `set_sample_rate()`, `set_gain()`, `capabilities() → DeviceCaps`, `info() → DeviceInfo` |
| 13.2 | `DeviceCaps` struct: gain stages (name, min, max, step), frequency range, sample rate range, optional features (amp, bias-T, AGC) |
| 13.3 | `DeviceInfo` struct: vendor, model, serial, firmware version, board revision |
| 13.4 | Refactor `src/hackrf.rs` to implement `SdrDevice`; no public HackRF types leak outside `src/device/hackrf/` |
| 13.5 | `BackendRegistry`: `detect_devices() → Vec<Box<dyn SdrDevice>>` scans all compiled-in backends |
| 13.6 | `App` holds `Box<dyn SdrDevice>`; all gain/freq/rate controls go through the trait; 0 regressions on HackRF |
| 13.7 | `MockDevice` implementing `SdrDevice` for all unit and integration tests; replaces any `libhackrf`-dependent test setup |

---

## Phase 14 — PortaPack / Mayhem Integration 🔲

**Goal:** Detect a PortaPack Mayhem device and display live telemetry (firmware version, platform model, RTC clock) in a dedicated panel. Auto-detection on startup; reconnect on unplug/replug.

**Protocol:** PortaPack Mayhem exposes a USB CDC/ACM serial interface (`/dev/ttyACM*`) independent of the HackRF RF USB interface.

```
host → device:   info\r
device → host:   Mayhem: 2.1.0\r\n
                 Platform: PortaPack H4M\r\n
                 ok\r\n
```

| Step | Description |
|---|---|
| 14.1 | `serialport = "4"` dep · `PortaPackState` in `state.rs` · 3 unit tests |
| 14.2 | `src/portapack.rs`: `find_portapack()`, `send_command()`, `PortaPackWorker` |
| 14.3 | `App` integration: spawn worker thread, add `'7'` preset key |
| 14.4 | `src/ui/portapack_panel.rs`: `PortaPackPanel` (connected / disconnected states) |
| 14.5 | Register panel, add `portapack` preset to `LayoutConfig`, update overlay |

---

## Phase 15 — HackRF Feature Pass 🔲

**Goal:** Get maximum value out of the existing HackRF backend before moving to new hardware. These features work on any SDR once the HAL is in place.

| Step | Description |
|---|---|
| 15.1 | **Band plan overlay:** configurable ITU region (1/2/3); band labels rendered on the spectrum canvas at correct frequency positions; built-in plans for amateur, broadcast, aviation, marine, ISM |
| 15.2 | Band plan as TOML: `~/.config/sdrtop/bandplan.toml`; user-extendable |
| 15.3 | **Frequency bookmarks:** `b` saves current frequency with optional label; `B` opens bookmark list; Enter jumps |
| 15.4 | **Signal markers:** `m` pins a named marker at the peak frequency within the visible band; markers persist in config |
| 15.5 | **IQ recording:** `R` starts/stops capture to SigMF format (`.sigmf-data` + `.sigmf-meta`); header shows duration and file size live |
| 15.6 | **Scan mode:** `S` sweeps a configured range (start / stop / step / dwell); spectrum fills in across the band as the sweep runs |
| 15.7 | **Metrics export:** `X` writes a CSV of rolling SNR, drop rate, and channel power to `~/.config/sdrtop/exports/` |

---

## Phase 16 — Polish & Production Readiness 🔲

| Step | Description |
|---|---|
| 16.1 | Startup UX: loading message, clean "no device" error with install hints |
| 16.2 | Terminal resize: forward `Event::Resize` as `AppEvent::Resize` |
| 16.3 | Mouse support: scroll over gain gauges to adjust; click device list items; scroll spectrum to tune |
| 16.4 | Panic hook: restore terminal unconditionally before printing panic message |
| 16.5 | Audit `unwrap()` calls; replace with `?` or `expect("reason")` |
| 16.6 | `--no-color` flag + `NO_COLOR` env var support |
| 16.7 | Performance: flamegraph pass, target ≥25 fps render, <30% CPU, <50 MB RSS |
| 16.8 | SSH / tmux validation: test all color depth modes on a headless connection |

---

## Phase 17 — Distribution & Community 🔲

| Step | Description |
|---|---|
| 17.1 | AUR packages: `sdrtop-git` and `sdrtop` |
| 17.2 | GitHub Actions CI: lint + test; release matrix for x86_64-linux and aarch64-linux (Raspberry Pi) |
| 17.3 | Nix flake with optional feature flags (`hackrf`, `rtlsdr`, `soapy`) |
| 17.4 | Man page via `clap_mangen` |
| 17.5 | `CONTRIBUTING.md`: how to add a new device backend (HAL contract, required tests, testing without hardware via `MockDevice`) |
| 17.6 | GitHub issue templates: bug report (includes `sdrtop --version` + OS + device), feature request, new backend request |

---

## Phase 18 — RTL-SDR Support ⏳

**Blocked on hardware.** Requires a physical RTL-SDR dongle for testing and tuner detection.

**Why:** RTL-SDR dongles (R820T, R828D, E4000) are the most widely owned SDR hardware in the world. Adding support immediately multiplies the potential user base by an order of magnitude.

| Step | Description |
|---|---|
| 18.1 | `librtlsdr` FFI in `src/device/rtlsdr/ffi.rs`; feature flag `rtlsdr` |
| 18.2 | `RtlSdrDevice` implementing `SdrDevice`; tuner-dependent gain steps via `capabilities()` |
| 18.3 | Tuner detection: E4000 / R820T / R828D / FC0012 / FC0013 — different gain steps and frequency limits per tuner |
| 18.4 | RTL-specific metrics: crystal PPM error, AGC mode, direct sampling mode |
| 18.5 | `RtlSdrPanel`: tuner type, PPM correction input, AGC toggle, bias-T (RTL-SDR Blog v3/v4) |
| 18.6 | `BackendRegistry` integration |

---

## Phase 19 — Airspy / Airspy HF+ Support ⏳

**Blocked on hardware.**

**Why:** Airspy devices are popular in the ham radio and HF monitoring community. The HF+ Discovery is one of the best budget HF-band receivers available.

| Step | Description |
|---|---|
| 19.1 | `libairspy` FFI; `AirspyDevice` implementing `SdrDevice` |
| 19.2 | Airspy-specific metrics: sensitivity preset (0–21), linearity preset, bias-T, packing mode |
| 19.3 | `libairspyhf` FFI; `AirspyHfDevice`: HF+/Discovery detection; mixer AGC, preamp state |
| 19.4 | `AirspyPanel` with device variant indicator and preset selector |
| 19.5 | `BackendRegistry` integration |

---

## Phase 20 — SoapySDR Backend ⏳

**Blocked on hardware.** At minimum a LimeSDR or SDRplay is needed to validate the implementation.

**Why:** SoapySDR is the universal hardware abstraction layer for SDR. One backend covers LimeSDR, bladeRF, SDRplay RSP series, PlutoSDR, USRP, FunCube Dongle, and dozens more without per-device FFI work.

| Step | Description |
|---|---|
| 20.1 | SoapySDR C API FFI in `src/device/soapy/ffi.rs`; optional feature flag `soapy` |
| 20.2 | `SoapyDevice` implementing `SdrDevice`; capabilities from SoapySDR channel info |
| 20.3 | Dynamic gain chain: iterate SoapySDR gain elements, render all stages in `RfChainPanel` |
| 20.4 | Device-specific settings panel: reads `SoapySDR::getSettingInfo()`, renders editable key-value pairs |
| 20.5 | Filter known devices (HackRF, RTL-SDR, Airspy) from SoapySDR enumeration — native backends take priority |
| 20.6 | `BackendRegistry` integration |

**Devices reachable via SoapySDR:** LimeSDR Mini · bladeRF 2.0 · SDRplay RSP1A / RSPdx · ADALM-PLUTO · USRP B200 · FunCube Dongle Pro+

---

## Phase 21 — Multi-Device Monitor ⏳

**Blocked on hardware.** Requires at least two different SDR devices simultaneously.

| Step | Description |
|---|---|
| 21.1 | `DeviceHandle` wraps `Box<dyn SdrDevice>` + FFT worker + metrics; `App` holds `Vec<DeviceHandle>` |
| 21.2 | Open all detected devices at startup; one polling task + FFT worker per device |
| 21.3 | Device list panel; `Tab` switches focus; `d` toggles panel visibility |
| 21.4 | `compare` preset: side-by-side spectrum panels on a shared frequency axis |
| 21.5 | Disconnect detection + 2-second reconnect watcher |

---

## Phase 22 — Advanced UI 🔲

| Step | Description |
|---|---|
| 22.1 | **Demodulation preview panel:** visual-only IF slice waveform (no audio) — AM envelope, FM deviation, raw IQ |
| 22.2 | **Signal history panel:** 60-second rolling sparkline of signal strength at current frequency |
| 22.3 | **Custom layout editor:** `L` enters resize mode; arrow keys adjust panel boundaries; saved as custom preset |
| 22.4 | **Mini-map:** low-resolution overview of ±(sample_rate/2) with a viewport box showing current zoom |
| 22.5 | **Constellation display:** IQ scatter on a Braille canvas; useful for visual modulation identification |
| 22.6 | Animated theme transitions: palette interpolation on theme switch |

---

## Phase 23 — Ecosystem 🔲

| Step | Description |
|---|---|
| 23.1 | Homebrew formula |
| 23.2 | Plugin hooks: external processes can subscribe to metrics via Unix socket |
| 23.3 | Scripting: `sdrtop --dump-json` streams metrics as newline-delimited JSON for piping to other tools |
| 23.4 | Community presets: shareable `LayoutConfig` + `Theme` bundles |

---

## Advanced Observer Mode 💡

> Rough concept — not committed.

The current observer mode reads sysfs and `/proc` — device identity, USB stats, owner process. It cannot access the IQ stream because `libhackrf` uses exclusive USB access.

**Option A — `sdrtopd` daemon:** Holds the device exclusively, re-exposes IQ over a Unix socket. Big compatibility problem with existing SDR software.

**Option B — `usbmon` passive capture:** Reads `/dev/usbmon*` without holding the device. Requires root; HackRF-specific USB parsing; fragile against protocol changes.

**Option C — Auto-recovery:** When the device is freed, automatically switch back to normal mode without user intervention. Best fit for the existing architecture. Practical starting point.

**Recommendation:** Option C first. A/B are research directions if there is community demand.

---

## Key risks

| Risk | Impact | Mitigation |
|---|---|---|
| `libhackrf` FFI struct differs across versions | crash / wrong data | check `hackrf_library_version()` at startup |
| HAL trait too narrow for SoapySDR capabilities | Phase 20 requires trait changes | design `DeviceCaps` and settings API generically in Phase 13 |
| FFT worker can't keep up at 20 Msps | stale spectrum | bounded drop channel; `FftFrame.stale` flag |
| Terminal lacks Braille / truecolor | broken display | `ColorDepth::detect()` at startup; ASCII fallback |
| USB disconnect mid-session | crash or hang | polling task catches error, recovers on reconnect |
| `main.rs` grows again | development friction | no file over 200 lines; clippy as CI gate |
| Mutex poisoning under panic | terminal in raw mode | `std::panic::set_hook` restores terminal (Phase 16.4) |
| SoapySDR ABI varies between distros | link errors | feature-gate behind `soapy`; document minimum version |
