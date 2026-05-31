# IMP-001 — Interactive Sample Rate Control

← [Home](../Home.md)

**Added:** 2026-05-28  
**Between phases:** 11 → 12  

---

## Why

Sample rate was set at startup (via config or CLI `--vga`) but could not be changed while the app was running. Every other RF parameter — frequency, LNA, VGA, AMP — had a live control. This was a gap that became obvious during hands-on testing.

---

## What changed

| File | Change |
|---|---|
| `src/state.rs` | Added `SampleRateInput` variant to `InputMode` enum |
| `src/app.rs` | `[S]` key switches to `SampleRateInput` mode; Enter validates (2–20 MHz), calls `device.set_sample_rate()`, updates `config_sample_rate` |
| `src/ui/footer.rs` | `[S] Rate` added to normal mode hint; new `SampleRateInput` arm shows live input buffer |
| `src/ui/overlay.rs` | `[S]` entry added; help text height bumped from 22 → 23 |

---

## Behaviour

- Press `[S]` → footer switches to input mode: `Sample rate (2–20 MHz): [▌]`
- Type a value in MHz (e.g. `8` or `12.5`), confirm with Enter
- Values outside 2–20 MHz are rejected with a log message
- On success: `config_sample_rate` is updated, the gains panel gauge reflects the new rate once throughput catches up
- `[Esc]` cancels without change
