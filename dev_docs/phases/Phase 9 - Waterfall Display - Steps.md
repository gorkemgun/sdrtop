# Phase 9 — Waterfall Display: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)

**Goal:** A scrolling 2D spectrum history rendered as rows of background-colored
terminal cells. Each row is one FFT frame; color encodes signal strength. The
waterfall appears below the spectrum in a combined `spectrum_waterfall` preset,
and alone in a `waterfall` preset.

**Prerequisite:** Phase 8 complete — `FftWorker` running, `FftFrame` in `SdrMetrics`.

---

## Correctness notes

**Bin-to-column mapping:** with N bins and W display columns, column `c` covers
bins `c*N/W .. (c+1)*N/W`. Use integer arithmetic throughout. Guard against an
empty range with `.max(bin_start + 1).min(n)`. Take the **maximum** bin value in
the range — peaks stay visible even when downsampling many bins into one column.

**Scroll direction:** newest row at the top. `WaterfallBuffer` stores rows
`push_front` / `pop_back`; the renderer walks `buf.rows.iter()` top-to-bottom,
so index 0 = latest frame = top row.

**Background color rendering:** `Span::styled(" ", Style::default().bg(color))`
— one space character per column, background colored. This works in truecolor and
256-color terminals without requiring any Canvas widget. The Paragraph widget fills
any remaining rows below the data with blank lines automatically.

**ColorDepth detection:** reading `COLORTERM` / `TERM` env vars is cheap but
should happen **once at startup**, not once per render. Store `ColorDepth` on
`WaterfallPanel::new()`. Terminal color depth does not change mid-session.

**Clone cost:** `SdrMetrics::clone()` copies the full waterfall buffer every render
frame. At 64 rows × 2048 bins × 4 bytes = 0.5 MB per clone, and at ≤10 fps, that
is ≤5 MB/s — acceptable on modern hardware. Phase 10 can introduce Arc-based sharing
if profiling reveals this as a bottleneck.

**`spectrum_waterfall` layout:** the existing `render_column` in `engine.rs`
applies `Constraint::Min(0)` to each panel in the same column. Two `Position::Body`
panels → equal vertical split. No engine changes needed.

**Paused state:** when `WaterfallBuffer::paused` is true, `push` is a no-op. The
display shows the frozen buffer unchanged. The panel title shows `[PAUSED]`. This
is independent of `SpectrumPanel`'s stale detection — spectrum can still be live
while the waterfall history is frozen.

---

## Dependency order

```
src/state.rs          WaterfallBuffer struct + field on SdrMetrics
    ↓
src/fft.rs            push row in same lock as FftFrame write
    ↓
src/palette.rs        ColorDepth + magnitude_to_color (+ tests)
src/main.rs           mod palette
    ↓
src/ui/waterfall.rs   WaterfallPanel implementing Panel
src/ui/mod.rs         pub use WaterfallPanel
    ↓
src/config.rs         waterfall + spectrum_waterfall presets
src/app.rs            register panel, key handlers (4 / 5 / w)
src/ui/overlay.rs     help text update
```

---

## Step 1 — `WaterfallBuffer` + `SdrMetrics` field + `FftWorker` push

**Files:** `src/state.rs`, `src/fft.rs`, `src/app.rs`

- [ ] **Add `WaterfallBuffer` to `src/state.rs`** — insert before `FftFrame`:

```rust
#[derive(Clone)]
pub struct WaterfallBuffer {
    pub rows: VecDeque<Vec<f32>>,
    pub max_rows: usize,
    pub paused: bool,
}

impl WaterfallBuffer {
    pub fn new(max_rows: usize) -> Self {
        Self { rows: VecDeque::new(), max_rows, paused: false }
    }

    pub fn push(&mut self, bins: Vec<f32>) {
        if self.paused { return; }
        if self.rows.len() >= self.max_rows {
            self.rows.pop_back();
        }
        self.rows.push_front(bins);
    }
}
```

- [ ] **Add `pub waterfall: WaterfallBuffer` to `SdrMetrics`** — after `last_fft_frame`:

```rust
pub last_fft_frame: Option<FftFrame>,
pub waterfall: WaterfallBuffer,
```

- [ ] **Add 3 unit tests** at the bottom of `src/state.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_adds_newest_row_first() {
        let mut buf = WaterfallBuffer::new(4);
        buf.push(vec![1.0, 2.0]);
        buf.push(vec![3.0, 4.0]);
        assert_eq!(buf.rows[0], vec![3.0, 4.0], "newest row should be at index 0");
        assert_eq!(buf.rows[1], vec![1.0, 2.0]);
    }

    #[test]
    fn push_respects_max_rows() {
        let mut buf = WaterfallBuffer::new(3);
        for i in 0..5u32 {
            buf.push(vec![i as f32]);
        }
        assert_eq!(buf.rows.len(), 3, "should not exceed max_rows");
    }

    #[test]
    fn paused_ignores_push() {
        let mut buf = WaterfallBuffer::new(4);
        buf.paused = true;
        buf.push(vec![1.0, 2.0]);
        assert!(buf.rows.is_empty(), "paused buffer should not accept new rows");
    }
}
```

- [ ] **Initialize in `App::new()`** in `src/app.rs`, inside the `SdrMetrics { ... }` literal, after `last_fft_frame: None`:

```rust
waterfall: crate::state::WaterfallBuffer::new(64),
```

- [ ] **Update `FftWorker::run` in `src/fft.rs`** — in the existing `if let Ok(mut m) = self.state.lock()` block, add the waterfall push **after** the `last_fft_frame` assignment:

```rust
if let Ok(mut m) = self.state.lock() {
    m.last_fft_frame = Some(FftFrame {
        bins_dbfs: smoothed.clone(),
        peak_hold: peak.clone(),
        noise_floor,
        center_freq_hz,
        sample_rate,
        timestamp: std::time::Instant::now(),
    });
    m.waterfall.push(smoothed.clone());   // ← add this line
}
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Run `cargo test state::tests`**. Expected: 3 tests pass.

---

## Step 2 — Color palette (`src/palette.rs`)

**Files:** Create `src/palette.rs`, update `src/main.rs`

- [ ] **Write `src/palette.rs`:**

```rust
use ratatui::style::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorDepth {
    TrueColor,
    Color256,
    Color16,
}

impl ColorDepth {
    pub fn detect() -> Self {
        let colorterm = std::env::var("COLORTERM").unwrap_or_default().to_lowercase();
        if colorterm == "truecolor" || colorterm == "24bit" {
            return Self::TrueColor;
        }
        let term = std::env::var("TERM").unwrap_or_default();
        if term.contains("256color") {
            return Self::Color256;
        }
        Self::Color16
    }
}

// 16-step xterm-256 gradient: dark blue (cold) → cyan → green → yellow → red (hot)
const PALETTE_256: [u8; 16] = [
     17,  // #00005f dark blue
     18,  // #000087
     19,  // #0000af
     21,  // #0000ff blue
     27,  // #005fff blue-cyan
     33,  // #0087ff
     51,  // #00ffff cyan
     46,  // #00ff00 green
     82,  // #5fff00
    118,  // #87ff00
    226,  // #ffff00 yellow
    220,  // #ffd700
    214,  // #ffaf00
    208,  // #ff8700
    202,  // #ff5f00
    196,  // #ff0000 red
];

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

// Piecewise linear gradient: dark blue → blue → cyan → green → yellow → red
fn truecolor_gradient(t: f32) -> (u8, u8, u8) {
    const STOPS: &[(f32, u8, u8, u8)] = &[
        (0.00,   0,   0, 128),
        (0.25,   0,   0, 255),
        (0.40,   0, 255, 255),
        (0.55,   0, 255,   0),
        (0.70, 255, 255,   0),
        (1.00, 255,   0,   0),
    ];
    for i in 0..STOPS.len() - 1 {
        let (t0, r0, g0, b0) = STOPS[i];
        let (t1, r1, g1, b1) = STOPS[i + 1];
        if t <= t1 {
            let s = (t - t0) / (t1 - t0);
            return (lerp(r0, r1, s), lerp(g0, g1, s), lerp(b0, b1, s));
        }
    }
    (255, 0, 0)
}

/// Map a dBFS value to a terminal color appropriate for the detected color depth.
pub fn magnitude_to_color(db: f32, db_min: f32, db_max: f32, depth: ColorDepth) -> Color {
    let t = ((db - db_min) / (db_max - db_min)).clamp(0.0, 1.0);
    match depth {
        ColorDepth::TrueColor => {
            let (r, g, b) = truecolor_gradient(t);
            Color::Rgb(r, g, b)
        }
        ColorDepth::Color256 => {
            let idx = ((t * 15.0) as usize).min(15);
            Color::Indexed(PALETTE_256[idx])
        }
        ColorDepth::Color16 => match (t * 3.0) as u8 {
            0 => Color::DarkGray,
            1 => Color::Blue,
            2 => Color::Cyan,
            _ => Color::White,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truecolor_cold_end_is_dark_blue() {
        let c = magnitude_to_color(-120.0, -120.0, 0.0, ColorDepth::TrueColor);
        // t = 0.0 → first stop: (0, 0, 128)
        assert_eq!(c, Color::Rgb(0, 0, 128));
    }

    #[test]
    fn truecolor_hot_end_is_red() {
        let c = magnitude_to_color(0.0, -120.0, 0.0, ColorDepth::TrueColor);
        assert_eq!(c, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn clamp_below_min_same_as_min() {
        let at_min  = magnitude_to_color(-120.0, -120.0, 0.0, ColorDepth::TrueColor);
        let below   = magnitude_to_color(-200.0, -120.0, 0.0, ColorDepth::TrueColor);
        assert_eq!(at_min, below, "values below db_min should clamp to cold end");
    }

    #[test]
    fn color16_covers_all_levels() {
        let cold = magnitude_to_color(-120.0, -120.0, 0.0, ColorDepth::Color16);
        let hot  = magnitude_to_color(   0.0, -120.0, 0.0, ColorDepth::Color16);
        assert_eq!(cold, Color::DarkGray);
        assert_eq!(hot,  Color::White);
    }
}
```

- [ ] **Add `mod palette;`** to `src/main.rs` alongside the existing module declarations:

```rust
mod palette;
```

- [ ] **Run `cargo test palette::tests`**. Expected: 4 tests pass.

---

## Step 3 — `WaterfallPanel` skeleton

**Files:** `src/ui/waterfall.rs`, `src/ui/mod.rs`

- [ ] **Replace the contents of `src/ui/waterfall.rs`** (currently empty stub):

```rust
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::palette::{magnitude_to_color, ColorDepth};
use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

const DB_MIN: f32 = -120.0;
const DB_MAX: f32 = 0.0;

pub struct WaterfallPanel {
    color_depth: ColorDepth,
}

impl WaterfallPanel {
    pub fn new() -> Self {
        Self { color_depth: ColorDepth::detect() }
    }
}

impl Panel for WaterfallPanel {
    fn name(&self) -> &'static str { "waterfall" }
    fn min_size(&self) -> (u16, u16) { (40, 5) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let buf = &state.waterfall;
        let title = if buf.paused { " Waterfall [PAUSED] " } else { " Waterfall " };

        if buf.rows.is_empty() {
            f.render_widget(
                Paragraph::new("Waiting for RX\u{2026}")
                    .block(Block::default().title(title).borders(Borders::ALL))
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::DarkGray)),
                area,
            );
            return;
        }

        // Full rendering added in Step 4
        f.render_widget(
            Block::default().title(title).borders(Borders::ALL),
            area,
        );
    }
}
```

- [ ] **Add to `src/ui/mod.rs`:**

```rust
pub use waterfall::WaterfallPanel;
```

The `pub mod waterfall;` line is already present from the Phase 4 stub. If
`WaterfallPanel` is not a unit struct, it cannot be `pub use`d with a default —
register it via `WaterfallPanel::new()` in `app.rs` (Step 5).

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 4 — Full colored-span rendering

**Files:** `src/ui/waterfall.rs`

- [ ] **Add imports** at the top of `src/ui/waterfall.rs`:

```rust
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
```

- [ ] **Replace the `// Full rendering added in Step 4` block** inside `render()` with:

```rust
        let block = Block::default().title(title).borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows_to_show = inner.height as usize;
        let cols = inner.width as usize;
        if cols == 0 { return; }

        let depth = self.color_depth;
        let mut lines: Vec<Line> = Vec::with_capacity(rows_to_show);

        for row_data in buf.rows.iter().take(rows_to_show) {
            let n = row_data.len();
            let mut spans: Vec<Span> = Vec::with_capacity(cols);
            for col in 0..cols {
                let bin_start = col * n / cols;
                let bin_end = (((col + 1) * n) / cols).max(bin_start + 1).min(n);
                let db = row_data[bin_start..bin_end]
                    .iter()
                    .cloned()
                    .fold(f32::NEG_INFINITY, f32::max);
                let color = magnitude_to_color(db, DB_MIN, DB_MAX, depth);
                spans.push(Span::styled(" ", Style::default().bg(color)));
            }
            lines.push(Line::from(spans));
        }

        f.render_widget(Paragraph::new(lines), inner);
```

Remove the now-unused `Paragraph` and `Alignment` imports if present — or leave
them; `cargo build` will emit warnings you can fix before Step 6 clippy.

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 5 — Presets, key bindings, overlay

**Files:** `src/config.rs`, `src/app.rs`, `src/ui/overlay.rs`

- [ ] **Add two presets to `src/config.rs`** in `default_config()`, after the `spectrum` preset:

```rust
let waterfall = PresetConfig {
    panels: vec![
        PanelSpec { name: "header".into(),    position: Top,    height: Some(3), width_pct: None },
        PanelSpec { name: "waterfall".into(),  position: Body,   height: None,    width_pct: None },
        PanelSpec { name: "log".into(),        position: Bottom, height: Some(5), width_pct: None },
        PanelSpec { name: "footer".into(),     position: Bottom, height: Some(3), width_pct: None },
    ],
};
let spectrum_waterfall = PresetConfig {
    panels: vec![
        PanelSpec { name: "header".into(),    position: Top,    height: Some(3), width_pct: None },
        PanelSpec { name: "spectrum".into(),   position: Body,   height: None,    width_pct: None },
        PanelSpec { name: "waterfall".into(),  position: Body,   height: None,    width_pct: None },
        PanelSpec { name: "log".into(),        position: Bottom, height: Some(5), width_pct: None },
        PanelSpec { name: "footer".into(),     position: Bottom, height: Some(3), width_pct: None },
    ],
};
```

Then register them:

```rust
presets.insert("waterfall".into(), waterfall);
presets.insert("spectrum_waterfall".into(), spectrum_waterfall);
```

- [ ] **Register `WaterfallPanel` in `src/app.rs`** alongside the other registrations:

```rust
registry.register(ui::WaterfallPanel::new());
```

- [ ] **Add key handlers** in the `InputMode::Normal` match arm in `src/app.rs`,
  after the `Char('3')` block:

```rust
KeyCode::Char('4') => {
    self.engine.set_preset("waterfall");
    self.state.lock().unwrap().push_log("Preset: waterfall");
}
KeyCode::Char('5') => {
    self.engine.set_preset("spectrum_waterfall");
    self.state.lock().unwrap().push_log("Preset: spectrum+waterfall");
}
KeyCode::Char('w') => {
    let mut m = self.state.lock().unwrap();
    m.waterfall.paused = !m.waterfall.paused;
    let state = if m.waterfall.paused { "paused" } else { "resumed" };
    m.push_log(format!("Waterfall {}", state));
}
```

- [ ] **Update the help overlay in `src/ui/overlay.rs`:**

After the `[3]` line, add:

```
 [4]        Preset: waterfall\n\
 [5]        Preset: spectrum+waterfall\n\
 [W]        Pause / resume waterfall\n\
```

Update the height in `centered_rect` from `18` to `21`:

```rust
let area = centered_rect(52, 21, f.size());
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 6 — Final validation

```bash
cargo build --release
cargo test
cargo clippy -- -D warnings
```

Expected: zero errors, zero warnings, all tests pass.

**Manual test checklist (HackRF connected):**

- [ ] Default preset (`minimal`) unchanged — all Phase 7/8 panels still work
- [ ] Press `3` → spectrum preset, spectrum renders as before
- [ ] Press `4` → waterfall preset
  - [ ] "Waiting for RX…" shown before streaming starts
- [ ] Press Space → RX starts
  - [ ] After ~2 s, colored rows appear in waterfall panel
  - [ ] Top row is newest; display scrolls upward as time passes
  - [ ] Color changes with signal strength (dark = weak, bright/red = strong)
- [ ] Press `5` → spectrum_waterfall preset
  - [ ] Spectrum in upper half, waterfall in lower half
  - [ ] Both update simultaneously
- [ ] Press `w` → `[PAUSED]` appears in waterfall title; rows freeze
- [ ] Press `w` again → paused clears; rows resume scrolling
- [ ] Press `f`, change frequency → frequency axis in spectrum updates; waterfall
  continues showing history from before the change
- [ ] Press `p` → cycles through all five presets: minimal → monitoring →
  spectrum → spectrum_waterfall → waterfall → minimal
- [ ] Press `1` → returns to minimal preset
- [ ] All Phase 5 / 7 / 8 keys still work in all presets
