# Phase 9 — Waterfall Display: Log

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Steps](Phase%209%20-%20Waterfall%20Display%20-%20Steps.md)

---

## What was built

| Component | File | Role |
|---|---|---|
| `WaterfallBuffer` | `src/state.rs` | Ring buffer of FFT rows; paused flag; push/pop discipline |
| Color palette | `src/palette.rs` | `ColorDepth` detection + `magnitude_to_color` |
| `WaterfallPanel` | `src/ui/waterfall.rs` | Full span-based rendering; idle state; paused title |
| `waterfall` preset | `src/config.rs` | header → waterfall (Body) → log → footer |
| `spectrum_waterfall` preset | `src/config.rs` | header → spectrum + waterfall (both Body) → log → footer |
| `4` / `5` / `w` keys | `src/app.rs` | Preset switches + waterfall pause toggle |
| Help overlay | `src/ui/overlay.rs` | Three new lines; height bumped 18 → 21 |

---

## WaterfallBuffer

```rust
pub struct WaterfallBuffer {
    pub rows: VecDeque<Vec<f32>>,   // index 0 = newest
    pub max_rows: usize,
    pub paused: bool,
}
```

`push_front` / `pop_back` keeps the newest row at index 0 so the renderer can
walk `buf.rows.iter()` top-to-bottom with no reversal. When `paused` is true,
`push` is a no-op — the deque is left unchanged and the display shows frozen
history.

The push happens inside the same `self.state.lock()` acquisition that writes
`last_fft_frame`, so the waterfall and the spectrum frame are always consistent.
FftWorker holds the lock for one assignment + one `push_front` / `pop_back` — a
few microseconds at most.

---

## Color palette (`src/palette.rs`)

Three rendering tiers, detected once at `WaterfallPanel::new()`:

| `ColorDepth` | Detection | Rendering |
|---|---|---|
| `TrueColor` | `COLORTERM=truecolor` or `24bit` | Piecewise RGB gradient (6 stops) |
| `Color256` | `TERM` contains `256color` | 16-step `PALETTE_256` xterm-256 lookup |
| `Color16` | fallback | 4 named colors (DarkGray / Blue / Cyan / White) |

**Truecolor gradient stops** (cold → hot):

| t | Color |
|---|---|
| 0.00 | `(0, 0, 128)` — dark blue |
| 0.25 | `(0, 0, 255)` — blue |
| 0.40 | `(0, 255, 255)` — cyan |
| 0.55 | `(0, 255, 0)` — green |
| 0.70 | `(255, 255, 0)` — yellow |
| 1.00 | `(255, 0, 0)` — red |

Linear interpolation (`lerp`) between stops. The `t` parameter is
`(db − db_min) / (db_max − db_min)` clamped to `[0, 1]`.

`ColorDepth::detect()` reads `COLORTERM` and `TERM` env vars. Called once in
`WaterfallPanel::new()` at app startup and stored on the struct — env vars do not
change mid-session.

---

## Rendering (`WaterfallPanel`)

The rendering does not use a Canvas widget. Each waterfall row becomes one
terminal row of `Span::styled(" ", Style::default().bg(color))` — one space
character per display column, background-colored. This works in truecolor, 256-
color, and 16-color terminals without any special widget support.

**Bin-to-column mapping** (integer arithmetic throughout):

```rust
let bin_start = col * n / cols;
let bin_end   = (((col + 1) * n) / cols).max(bin_start + 1).min(n);
let db = row_data[bin_start..bin_end]
    .iter().cloned().fold(f32::NEG_INFINITY, f32::max);
```

`.max(bin_start + 1)` guards against an empty range when `cols > n` (upscaling).
`.min(n)` guards against overflow when `col == cols - 1`.
Taking the **maximum** across the range preserves peaks even when many bins map
to one column.

**Layout:** `Block::inner(area)` gives the drawable area inside the border.
The renderer uses `inner.height` rows and `inner.width` columns — never `area`
dimensions, which include the border.

---

## `spectrum_waterfall` preset — no engine changes

The existing `render_column` function applies `Constraint::Min(0)` to every panel
in the same body column. Two `Position::Body` panels → equal vertical split.
No engine changes were needed.

---

## Overlay height fix

Adding `[4]`, `[5]`, `[W]` increased the help text by 3 lines. The
`centered_rect` call was updated from `height = 18` to `height = 21` so all
lines fit inside the box without clipping.

---

## Deviations from plan

- **Steps 3 and 4 merged:** the step file planned a skeleton write (Step 3)
  followed by replacing a placeholder with full rendering (Step 4). Since all
  rendering lives in one `render()` function, both were written in a single
  `waterfall.rs` file creation — no intermediate broken state and no need to
  re-edit the same function twice.

---

## Final state

```
cargo build --release   → Finished (0 errors, 0 warnings)
cargo test              → 27 passed, 0 failed
cargo clippy -D warnings → 0 findings
```
