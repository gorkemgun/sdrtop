# Phase 12d — Codebase Modularization — Log

← [Roadmap](../Roadmap.md) | [Steps](Phase%2012d%20-%20Codebase%20Modularization%20-%20Steps.md)

## Status: ✅ Done

---

## Deviations from plan

### `state.rs` → `state/` conversion required two separate sed passes

The Steps doc anticipated a single rename pass for all `m.<field>` accesses.
In practice, UI panel files use `state.<field>` (they receive `&SdrMetrics` as a
parameter named `state`, not `m`), while tasks and app.rs use `m.<field>` (from
inside mutex lock guards). Two separate sed passes were required — one per naming
convention — rather than the single pass described in the plan.

### Waterfall `state.waterfall` → double-nesting trap

The bare `state.waterfall` in panel files previously referred to `WaterfallBuffer`
directly. After the first sed pass renamed `state.waterfall_db_min` to
`state.waterfall.db_min`, the subsequent catch-all replacement
`state.waterfall → state.waterfall.buffer` also hit the newly renamed
`state.waterfall.db_min`, producing `state.waterfall.buffer.db_min`.

`db_min`, `scroll_offset`, `cursor_freq`, and `last_fft` live on `WaterfallState`
(the outer struct), not on `WaterfallBuffer` (the inner `.buffer` field).
A third targeted pass un-nested these four fields back to `state.waterfall.X`.

**Lesson:** When doing layered renames, rename specific sub-paths before the
broad catch-all, or use a non-word-boundary pattern for the catch-all.

### Chained `.unwrap_or_else(|e| e.into_inner()).field` not matched by sed

The `sed` pattern `\bm\.field\b` only matched standalone `m.field` accesses.
In `app.rs`, several one-liners chain the lock guard directly:
`self.state.lock().unwrap_or_else(|e| e.into_inner()).input_buf.pop()`.
These were not renamed by the bulk pass. Python string replacement was used for
the remaining handful of cases.

### `pub(super)` required on `App` fields for `builder.rs`

The Steps doc stated that submodules can access private fields of parent-module
types. In Rust this is correct in principle, but `builder.rs` constructs
`App { ... }` struct literals which require field visibility. Marking fields
`pub(super)` was sufficient — builder.rs (as `app::builder`) can see `pub(super)`
items from its parent `app`.

### `app/input.rs` ended up at 556 lines — above the 250-line target

The Steps doc targeted ≤250 lines for `input.rs`. The actual count is 556 lines.
The key handler logic is genuinely that large — each of the 8 sub-handlers
(global, spectrum focus, waterfall focus, freq input, sample-rate input,
marker input, and two dispatch helpers) is under 100 lines individually. The
file is long but not complex. Splitting further (e.g. one file per handler)
would add boilerplate without clarity. The 250-line target was aspirational.

### `WaterfallBuffer` unexported from `state`

The Steps doc planned to re-export `WaterfallBuffer` from `state/mod.rs`.
After the split, `WaterfallBuffer` is only accessed via `state.waterfall.buffer`
— no external code imports the type by name. The re-export was dropped as unused.

### `LOG_MAX_ENTRIES` unexported from `state`

Same situation. The constant is used only inside `state/ui.rs`. The re-export
from `state/mod.rs` was removed after the compiler flagged it as unused.

---

## Key decisions

**`SdrMetrics` keeps `push_log()` and `reset_to_defaults()` as top-level methods**
rather than delegating to sub-structs. Callers always have `&mut SdrMetrics`, not
`&mut UiState`, so a top-level `m.push_log(...)` is more ergonomic than
`m.ui.push_log(...)` at every call site.

**Accumulators are `pub(crate)`**, not `pub`. The hardware callback (same crate)
still writes to `m.acc.drops` etc., but UI panels cannot accidentally read raw
accumulator data. This enforces the snapshot pattern: polling task reads, resets,
and publishes derived metrics; UI reads only the derived values.

**`input.rs` uses a `KeyAction` enum** (`Continue` / `Quit`) rather than
returning `io::Result<()>`. The `q` key cannot call `save_config()` directly
because that requires `&self` on `App`. The enum signals intent back to `run()`,
which owns `save_config()`. Clean separation: input handlers know nothing about
`App` internals.

**`handle_normal()` dispatches by focused panel before matching keys**, not the
other way around. The old code had one flat `match key.code` with dozens of guard
clauses (`if self.engine.focused_panel_name() == Some("spectrum")`). The new code
checks the panel first, then routes to a dedicated sub-handler. Adding a new panel
with focus keys requires one new function and one new arm in the dispatch match.

**`builder.rs` is a submodule (`app::builder`), not a separate free module.**
Both `new_normal` and `new_observer` are `impl App` methods defined in builder.rs.
This lets them call `Self::build_ui(...)` naturally and construct `Self { ... }`
with struct literal syntax, without needing a separate `App::from_parts()`
constructor.

**`signal/mod.rs` keeps `dsp` private** (`mod dsp;` not `pub mod dsp;`).
`WindowFn` and `compute_window` are implementation details of `FftWorker` — no
external code needs them. Only `FftWorker` is re-exported.

---

## Files changed

| File | Change |
|---|---|
| `src/app.rs` | Deleted |
| `src/app/mod.rs` | New — App struct, `run()`, `draw()`, `save_config()`, `App::new()` dispatcher |
| `src/app/builder.rs` | New — `App::new_normal()`, `App::new_observer()`, `App::build_ui()` |
| `src/app/input.rs` | New — `handle_key()` + 7 sub-handlers by mode and panel focus |
| `src/fft.rs` | Deleted — moved to `src/signal/fft.rs` |
| `src/dsp.rs` | Deleted — moved to `src/signal/dsp.rs` |
| `src/signal/mod.rs` | New — re-exports `FftWorker` |
| `src/signal/fft.rs` | Moved from `src/fft.rs`; updated import: `super::dsp` |
| `src/signal/dsp.rs` | Moved from `src/dsp.rs`; unchanged |
| `src/tasks.rs` | Deleted |
| `src/tasks/mod.rs` | New — re-exports + `fmt_duration` + test |
| `src/tasks/rx.rs` | Split from `tasks.rs`; fields renamed to `m.radio.*`, `m.acc.*` etc. |
| `src/tasks/observer.rs` | Split from `tasks.rs`; fields renamed to `m.observer.*` |
| `src/tasks/system.rs` | Split from `tasks.rs`; fields renamed to `m.system.*` |
| `src/state.rs` | Deleted |
| `src/state/mod.rs` | New — `SdrMetrics` container + `push_log()` + `reset_to_defaults()` |
| `src/state/radio.rs` | New — `RadioState` |
| `src/state/signal.rs` | New — `SignalState` |
| `src/state/iq.rs` | New — `IqState` |
| `src/state/observer.rs` | New — `ObserverState` |
| `src/state/spectrum.rs` | New — `SpectrumState` + `SpectrumMarker` |
| `src/state/waterfall.rs` | New — `WaterfallState` + `WaterfallBuffer` + `FftFrame` + tests |
| `src/state/system.rs` | New — `SystemState` |
| `src/state/ui.rs` | New — `UiState` + `InputMode` + `push_log()` |
| `src/state/acc.rs` | New — `Accumulators` (pub(crate)) |
| `src/main.rs` | Removed `mod fft; mod dsp;`; added `mod signal;` |
| `src/hardware/device.rs` | All `m.acc_*` → `m.acc.*`; `m.bytes_since_last_poll` → `m.radio.*` etc. |
| `src/signal/fft.rs` | All `m.frequency` → `m.radio.frequency` etc.; `m.waterfall.push` → `m.waterfall.buffer.push` |
| `src/ui/*.rs` (18 files) | All `state.<field>` renamed to `state.<substruct>.<field>` |

---

## Test results

```
running 80 tests
... all pass

cargo build  →  Finished (0 errors, 0 warnings)
cargo test   →  80 passed; 0 failed; 0 ignored
```

---

## Before / after

| Metric | Before | After |
|---|---|---|
| `src/app.rs` | 1006 lines | deleted |
| `app/mod.rs` | — | 138 lines |
| `app/builder.rs` | — | 251 lines |
| `app/input.rs` | — | 556 lines |
| `src/state.rs` | 278 lines | deleted |
| `state/` total | — | 9 files, ~60 lines each |
| `src/tasks.rs` | 285 lines | deleted |
| `tasks/` total | — | 3 + mod, ~60–142 lines each |
| `src/fft.rs` | 267 lines | moved to `signal/fft.rs` |
| `src/dsp.rs` | 57 lines | moved to `signal/dsp.rs` |
| `SdrMetrics` public fields | 50+ flat fields | 8 named sub-structs + 1 `pub(crate)` |
| `run()` method | ~550 lines | ~30 lines |
| Key handlers in one match | 1 × 400-line match | 8 focused functions |
