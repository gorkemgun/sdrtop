# sdrtop — Development Hub

> btop-inspired TUI for HackRF One & PortaPack SDR devices, written in Rust.

**Stack:** Rust · ratatui · libhackrf FFI · tokio · rustfft · crossbeam  
**Progress:** Phase 12 of 17 complete · 4 bugs tracked · 3 improvements logged

---

## Navigation

| Document                               | Purpose                                |
| -------------------------------------- | -------------------------------------- |
| [Roadmap](Roadmap.md)                  | Vision, upcoming phases (13–17), risks |
| [Changelog](CHANGELOG.md)              | Chronological milestone history        |
| [Bug tracker](bugs/README.md)          | Known, active, and resolved bugs       |
| [Improvements](improvements/README.md) | Out-of-phase additions                 |

---

## Phase progress

| # | Title | Status | Docs |
|---|---|---|---|
| 1 | Device discovery & basic info | ✅ Done | [steps](<phases/Phase 1 - Device Discovery - Steps.md>) · [log](<phases/Phase 1 - Device Discovery - Log.md>) |
| 2 | Telemetry polling & USB throughput | ✅ Done | [steps](<phases/Phase 2 - Telemetry Polling - Steps.md>) · [log](<phases/Phase 2 - Telemetry Polling - Log.md>) |
| 3 | TUI dashboard | ✅ Done | [steps](<phases/Phase 3 - TUI Dashboard - Steps.md>) · [log](<phases/Phase 3 - TUI Dashboard - Log.md>) |
| 4 | Architecture refactor | ✅ Done | [steps](<phases/Phase 4 - Architecture Refactor - Steps.md>) · [log](<phases/Phase 4 - Architecture Refactor - Log.md>) |
| 5 | Interactive controls | ✅ Done | [steps](<phases/Phase 5 - Interactive Controls - Steps.md>) · [log](<phases/Phase 5 - Interactive Controls - Log.md>) |
| 6 | Dashboard engine (panel system, presets) | ✅ Done | [steps](<phases/Phase 6 - Dashboard Engine - Steps.md>) · [log](<phases/Phase 6 - Dashboard Engine - Log.md>) |
| 7 | Hardware health panels | ✅ Done | [steps](<phases/Phase 7 - Hardware Health Panels - Steps.md>) · [log](<phases/Phase 7 - Hardware Health Panels - Log.md>) |
| 8 | FFT spectrum analyzer | ✅ Done | [8a](<phases/Phase 8a - FFT Pipeline - Steps.md>) · [8b](<phases/Phase 8b - Spectrum Display - Steps.md>) · [log](<phases/Phase 8 - FFT Spectrum Analyzer - Log.md>) |
| 9 | Waterfall display | ✅ Done | [steps](<phases/Phase 9 - Waterfall Display - Steps.md>) · [log](<phases/Phase 9 - Waterfall Display - Log.md>) |
| 10 | Configuration & persistence | ✅ Done | [steps](<phases/Phase 10 - Configuration & Persistence - Steps.md>) · [log](<phases/Phase 10 - Configuration & Persistence - Log.md>) |
| 11 | HackRF deep diagnostics | ✅ Done | [steps](<phases/Phase 11 - HackRF Deep Diagnostics - Steps.md>) · [log](<phases/Phase 11 - HackRF Deep Diagnostics - Log.md>) |
| 12 | UI/UX polish & theme system | ✅ Done | [12a](<phases/Phase 12a - Theme Foundation - Steps.md>) · [12b](<phases/Phase 12b - Panel Visual Updates - Steps.md>) · [12c](<phases/Phase 12c - Header Footer Focus - Steps.md>) · [log](<phases/Phase 12 - UI UX Polish Theme System - Log.md>) |
| 13 | PortaPack / Mayhem integration | 🔲 Planned | [steps](<phases/Phase 13 - PortaPack Mayhem Integration - Steps.md>) |
| 14 | Multi-device support | 🔲 Planned | — |
| 15 | Polish & production readiness | 🔲 Planned | — |
| 16 | Distribution & community | 🔲 Planned | — |
| 17 | Advanced observer mode | 💡 Idea | — |

---

## Out-of-phase improvements

Additions made between planned phases — not bugs, not roadmap items.

| ID | Title | Between | Status |
|---|---|---|---|
| [IMP-001](improvements/imp-001-sample-rate-control.md) | Interactive sample rate control (`[S]` key) | 11→12 | ✅ Done |
| [IMP-002](improvements/imp-002-observer-mode.md) · [log](improvements/imp-002-observer-mode-log.md) | Observer mode — monitor while another app holds the HackRF | 11→12 | ✅ Done |
| [IMP-003](improvements/imp-003-spectrum-waterfall-ui-fixes.md) | Spectrum & waterfall UI fixes (border, freq labels, axis alignment, dBFS legend) | 12→13 | ✅ Done |

---

## Docs conventions

| File type | When written | Purpose |
|---|---|---|
| `Phase N - … - Steps.md` | Before implementation | Intended approach, sub-steps, expected tests |
| `Phase N - … - Log.md` | After implementation | What actually happened, deviations, key decisions |
| `bug-NNN-….md` | At discovery | Symptom, root cause, fix, regression test |
| `imp-NNN-….md` | After completion | Why, what changed, before/after behaviour |

Both Steps and Log exist for every completed phase. For planned phases, only Steps exist.
