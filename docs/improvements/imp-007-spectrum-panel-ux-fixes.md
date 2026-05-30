# IMP-007 — Spectrum panel UX fixes

← [Home](../Home.md)

**Added:** 2026-05-30  
**Between phases:** 12 → 13

---

## Why

Two usability problems in the spectrum panel were reported after IMP-006 shipped:

1. The `◀ MHz ▶` tuning indicator was visually off-centre — it appeared shifted left whenever the right-side info text (`cur: … dBFS  J/K` or `step … [/]`) was present.
2. Holding `J` or `K` to move the cursor caused the whole application to lag noticeably and occasionally freeze. The event loop redraws on every keyboard event; at keyboard-repeat rates (30–50 events/sec) this meant 30–50 full canvas renders per second.

---

## What changed

| File | Change |
|---|---|
| `src/ui/spectrum.rs` | Tuning indicator arm calculation fixed: left arm now ignores `right_info` length |
| `src/app.rs` | Frame rate cap added to `run()`: renders at most every 33 ms (~30 fps) |

---

## Fix 1 — Tuning indicator centering

### Before

```rust
let fixed = 2 + freq_str.len() + right_info.len();
let arm   = (ind_area.width as usize).saturating_sub(fixed) / 2;
// Layout: [arm dashes] ◀ freq ▶ [arm dashes] [right_info]
```

`arm` was calculated by treating `◀ freq ▶` + `right_info` as one block and splitting the leftover evenly. This gave equal left and right *dash arms*, but the right arm was then consumed by `right_info`, so the `◀ freq ▶` block ended up left of centre.

### After

```rust
let center_len = 2 + freq_str.len();   // only ◀ + freq_str + ▶
let left_arm   = (ind_area.width as usize).saturating_sub(center_len) / 2;
let right_arm  = (ind_area.width as usize)
    .saturating_sub(left_arm + center_len + right_info.len());
```

`left_arm` is now derived from the full panel width and the centre block only. `right_arm` fills whatever space is left between the centre block and `right_info`. The `◀ freq ▶` block sits at true centre regardless of how long `right_info` is.

---

## Fix 2 — Frame rate cap

### Before

```
loop:
  clone state + terminal.draw()   ← on every iteration
  events.recv()                   ← blocks until next event
  handle event
```

With keyboard repeat at 30–50 events/sec, `terminal.draw()` was called 30–50 times/sec. Each draw clones `SdrMetrics` (including the waterfall buffer) and renders the canvas.

### After

```
draw initial frame

loop:
  events.recv()
  handle event
  if Key:  redraw only if ≥ 33 ms since last draw
  if Tick: always redraw  (idle / background metric refresh)
```

Key events cap the frame rate to ~30 fps. A single key press still redraws immediately on the next Tick (≤ 100 ms). Tick events continue to fire every 100 ms, keeping background metric panels (throughput, gain, log) responsive when the keyboard is idle.
