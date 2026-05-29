# sdrtop — Roadmap

← [Home](Home.md)

---

## Vision

`sdrtop` is a terminal-based SDR monitor in the spirit of `btop`: visually rich, fully interactive, and genuinely useful as a daily driver. The end state is an app that an RF engineer opens instead of `hackrf_info` + `gqrx` + a scratchpad — one tool that shows everything, lets you tune everything, and gets out of the way.

**Primary target:** HackRF One and PortaPack H1/H2 with Mayhem firmware.  
**Architecture:** designed for future extensibility to RTL-SDR, LimeSDR, Airspy — but not a priority until the HackRF experience is complete.

---

## Current status

12 of 17 phases complete. The core RF toolchain — FFT pipeline, spectrum, waterfall, theme system, interactive controls — is done. Next up: PortaPack integration and multi-device support.

| Phase | Title | Status |
|---|---|---|
| 1–12 | Core toolchain (device → FFT → waterfall → themes) | ✅ Done |
| 13 | PortaPack / Mayhem integration | 🔲 Next |
| 14 | Multi-device support | 🔲 Planned |
| 15 | Polish & production readiness | 🔲 Planned |
| 16 | Distribution & community | 🔲 Planned |
| 17 | Advanced observer mode | 💡 Idea |

Full phase history with per-phase logs: [Home](Home.md#phase-progress).

---

## Technology stack

| Concern | Choice | Notes |
|---|---|---|
| Language | Rust stable | |
| TUI | `ratatui 0.26+` | layout, widgets, Braille canvas |
| Hardware FFI | `libhackrf` via `pkg-config` | custom FFI — bypasses broken `hackrf` 0.0.1 crate |
| Async runtime | `tokio` | background polling & FFT task |
| FFT | `rustfft 6` | pure-Rust, no C dependency |
| Config | `toml 0.8` + `serde 1` | `~/.config/sdrtop/config.toml` |
| CLI args | `clap 4` (derive) | |
| Channels | `crossbeam-channel 0.5` | lock-free IQ sample handoff |

---

## Phase 13 — PortaPack / Mayhem Integration 🔲

**Goal:** Detect a PortaPack Mayhem device and display live telemetry (firmware version, platform model, RTC clock) in a dedicated panel. Auto-detection on startup; reconnect on unplug/replug.

**Steps:**

| Step | Description |
|---|---|
| 13.1 | `serialport = "4"` dep · `PortaPackState` in `state.rs` · 3 unit tests |
| 13.2 | `src/portapack.rs`: `find_portapack()`, `send_command()`, `PortaPackWorker` · 4 unit tests |
| 13.3 | `App` integration: spawn worker thread, add `'7'` preset key |
| 13.4 | `src/ui/portapack_panel.rs`: `PortaPackPanel` (connected/disconnected states) |
| 13.5 | Register panel, add `portapack` preset to `LayoutConfig`, update overlay |

**Protocol:** PortaPack Mayhem exposes a USB CDC/ACM serial interface (`/dev/ttyACM*`) independent of the HackRF RF USB interface.

```
host → device:   info\r
device → host:   Mayhem: 2.1.0\r\n
                 Platform: PortaPack H2\r\n
                 ok\r\n
```

Detection: open `/dev/ttyACM*`, send `info\r`, check response for `"Mayhem"`.

---

## Phase 14 — Multi-Device Support 🔲

**Goal:** Multiple HackRF devices monitored simultaneously; `Tab` switches focus.

| Step | Description |
|---|---|
| 14.1 | Introduce `DeviceHandle` struct; refactor `App` to hold `Vec<DeviceHandle>` |
| 14.2 | Open all connected devices at startup; spawn one polling task + FFT worker per device |
| 14.3 | Device list panel (`src/ui/device_list.rs`); `d` key toggles; `Tab` changes focus |
| 14.4 | Disconnect detection: mark device offline, stop FFT worker |
| 14.5 | Reconnect detection via 2-second watcher task |

---

## Phase 15 — Polish & Production Readiness 🔲

| Step | Description |
|---|---|
| 15.1 | Startup UX: loading message, clean "no device" error |
| 15.2 | Terminal resize: forward `Event::Resize` as `AppEvent::Resize` |
| 15.3 | Mouse support: scroll over gauges, click device list |
| 15.4 | Panic hook: restore terminal unconditionally before printing panic message |
| 15.5 | Audit `unwrap()` calls; replace with `?` or `expect("reason")` |
| 15.6 | `--no-color` flag + `NO_COLOR` env var support |
| 15.7 | Performance: flamegraph, target ≥25 fps render, <30% CPU, <50 MB RSS |
| 15.8 | Integration test harness with `libhackrf_mock.so` |

---

## Phase 16 — Distribution & Community 🔲

| Step | Description |
|---|---|
| 16.1 | AUR packages: `sdrtop-git` and `sdrtop` |
| 16.2 | GitHub Actions CI (lint + test) and release matrix (4 targets) |
| 16.3 | Nix flake |
| 16.4 | Homebrew formula |
| 16.5 | `README.md`, `CONTRIBUTING.md`, man page via `clap` |

---

## Phase 17 — Advanced Observer Mode 💡

> Rough concept — not committed. Exists so the idea is not lost.

**The gap:** The current observer mode reads sysfs and `/proc` only — device identity, USB stats, owner process info. It cannot see the IQ stream or the owner app's frequency/gain. That is a fundamental USB constraint: libhackrf uses exclusive access.

| What we want | Why we can't have it now |
|---|---|
| Live IQ spectrum / waterfall | Exclusive USB — another app holds the device |
| Current frequency & gain of owner | Lives inside SDR++, not exposed |
| Auto-recovery when device is freed | Not implemented — user must restart |

**Options:**

**Option A — `sdrtopd` daemon (powerful, breaking)**  
A background daemon holds the HackRF exclusively and re-exposes IQ over a Unix socket / shared memory ring buffer. Other apps connect to the daemon instead of USB. Big compatibility problem.

**Option B — `usbmon` passive capture (interesting, fragile)**  
`usbmon` captures USB traffic without holding the device. A privileged daemon reads `/dev/usbmon*`, parses HackRF's bulk transfer format, reconstructs IQ. Requires root + HackRF-specific parsing; will break on protocol changes.

**Option C — Auto-recovery only (minimal, practical)**  
Keep the sysfs-based observer, add a watcher task that polls every 2 seconds. When the device is freed, restart in normal mode without user intervention. Best fit for the existing architecture.

**Recommendation:** Start with Option C. Options A/B are research directions.

---

## Key risks

| Risk | Impact | Mitigation |
|---|---|---|
| `libhackrf` FFI struct differs across versions | crash / wrong data | check `hackrf_library_version()` at startup |
| FFT worker can't keep up at 20 Msps | stale spectrum | bounded drop channel; `FftFrame.stale` flag |
| Terminal lacks Braille / truecolor | broken display | `ColorDepth::detect()` at startup; ASCII fallback |
| USB disconnect mid-session | crash or hang | polling task catches error, recovers on reconnect |
| `main.rs` grows again | development friction | no file over 200 lines; clippy as CI gate |
| Mutex poisoning under panic | terminal in raw mode | `std::panic::set_hook` restores terminal (Phase 15.4) |
