# Phase 8b — Spectrum Display: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)

**Goal:** Render the `FftFrame` produced by Phase 8a as a live spectrum panel.
`SpectrumPanel` implements the Phase 6 `Panel` trait and plugs into the existing
registry and layout engine with no engine changes.

**Prerequisite:** Phase 8a complete — `FftFrame` in `SdrMetrics`, FftWorker running.

---

## Correctness notes

**Bin-to-column mapping:** with N bins and W display columns, each column covers
N/W bins. Display the maximum bin value across each column's range — this gives a
visually crisp result without averaging away peaks.

**Canvas coordinate space:** `Canvas::x_bounds([0.0, N])`, `y_bounds([db_min, db_max])`.
Drawing a `Line` from `(x, db_min)` to `(x, bin_db)` at each bin index x fills a
vertical bar. The Canvas maps logical coordinates to Braille cells automatically.

**Stale detection:** compare `frame.timestamp.elapsed()` to 500 ms. If the FFT
worker has stopped producing frames (e.g., RX stopped), the display should say so
rather than showing frozen data silently.

**Key bindings `n`/`w` (FFT size / window):** require restarting FftWorker with
new parameters — deferred. In Phase 8b the FFT size is fixed (2048, Hann). The
overlay notes these as "(Phase 9)" placeholders.

---

## Step 1 — `SpectrumPanel` skeleton

**Files:** `src/ui/spectrum.rs`, `src/ui/mod.rs`

- [ ] **Write `src/ui/spectrum.rs`** — idle state only, no data rendering yet:

```rust
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

const DB_MIN: f32 = -120.0;
const DB_MAX: f32 = 0.0;

pub struct SpectrumPanel;

impl Panel for SpectrumPanel {
    fn name(&self) -> &'static str { "spectrum" }
    fn min_size(&self) -> (u16, u16) { (40, 10) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let stale = state.last_fft_frame.as_ref().map(|fr| {
            fr.timestamp.elapsed() > std::time::Duration::from_millis(500)
        }).unwrap_or(false);

        let title = if stale { " Spectrum [STALE] " } else { " Spectrum " };

        match state.last_fft_frame.as_ref() {
            None => {
                f.render_widget(
                    Paragraph::new("Waiting for RX…")
                        .block(Block::default().title(title).borders(Borders::ALL))
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(Color::DarkGray)),
                    area,
                );
            }
            Some(_frame) => {
                // Full rendering added in Steps 2–5
                f.render_widget(
                    Block::default().title(title).borders(Borders::ALL),
                    area,
                );
            }
        }
    }
}
```

- [ ] **Add to `src/ui/mod.rs`:**

```rust
pub mod spectrum;  // already exists as stub — replace its contents
pub use spectrum::SpectrumPanel;
```

If `spectrum` is already in the `pub mod` list, just add the `pub use` line.

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 2 — Canvas bar rendering

**Files:** `src/ui/spectrum.rs`

Replace the `Some(_frame)` arm with full Canvas-based bar rendering.

- [ ] **Add imports to `src/ui/spectrum.rs`**:

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Points},
        Block, Borders, Paragraph,
    },
    Frame,
};
```

- [ ] **Replace the `Some(_frame)` arm** in `render()`:

```rust
Some(frame) => {
    // Split: left 6 cols = dBFS labels, right = canvas + freq axis
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(6), Constraint::Min(1)])
        .split(area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(4), Constraint::Length(1)])
        .split(cols[1]);

    let canvas_area = rows[0];
    let freq_area   = rows[1];
    let db_area     = cols[0];

    let n = frame.bins_dbfs.len() as f64;
    let title_style = if stale {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    };

    // Spectrum canvas
    let bins = frame.bins_dbfs.clone();
    let peaks = frame.peak_hold.clone();
    f.render_widget(
        Canvas::default()
            .block(
                Block::default()
                    .title(Span::styled(title, title_style))
                    .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM),
            )
            .x_bounds([0.0, n])
            .y_bounds([DB_MIN as f64, DB_MAX as f64])
            .paint(move |ctx| {
                let bar_color = Color::Green;
                for (i, &db) in bins.iter().enumerate() {
                    let y = (db.clamp(DB_MIN, DB_MAX)) as f64;
                    ctx.draw(&CanvasLine {
                        x1: i as f64, y1: DB_MIN as f64,
                        x2: i as f64, y2: y,
                        color: bar_color,
                    });
                }
                // Peak hold as individual points
                for (i, &db) in peaks.iter().enumerate() {
                    let y = db.clamp(DB_MIN, DB_MAX) as f64;
                    ctx.draw(&Points {
                        coords: &[(i as f64, y)],
                        color: Color::Yellow,
                    });
                }
            }),
        canvas_area,
    );

    // Frequency axis labels (1 row below canvas)
    let bw = frame.sample_rate;
    let left_hz = frame.center_freq_hz as f64 - bw / 2.0;
    let freq_labels: Vec<String> = (0..=4)
        .map(|i| format!("{:.2}M", (left_hz + bw * i as f64 / 4.0) / 1_000_000.0))
        .collect();
    f.render_widget(
        Paragraph::new(Span::raw(
            format!("{:<12}{:<12}{:<12}{:<12}{}", freq_labels[0], freq_labels[1],
                freq_labels[2], freq_labels[3], freq_labels[4])
        ))
        .style(Style::default().fg(Color::DarkGray)),
        freq_area,
    );

    // dBFS labels (left column, 5 levels top to bottom)
    let db_text: String = (0..=4)
        .map(|i| {
            let db = DB_MAX - (DB_MAX - DB_MIN) * i as f32 / 4.0;
            format!("{:+4.0}\n", db)
        })
        .collect();
    f.render_widget(
        Paragraph::new(db_text)
            .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM))
            .style(Style::default().fg(Color::DarkGray)),
        db_area,
    );
}
```

- [ ] **Run `cargo build`**. Fix any borrow/lifetime issues — `bins` and `peaks`
  are cloned before the closure to avoid capturing `frame` by reference across
  the `paint` closure boundary.

---

## Step 3 — Noise floor line

**Files:** `src/ui/spectrum.rs`

- [ ] **Inside the `paint` closure**, after the peak-hold loop, add:

```rust
// Noise floor as a horizontal line across the full spectrum
let nf = frame.noise_floor.clamp(DB_MIN, DB_MAX) as f64;
ctx.draw(&CanvasLine {
    x1: 0.0, y1: nf,
    x2: n,   y2: nf,
    color: Color::DarkGray,
});
```

The `noise_floor` value needs to be captured in the closure. Clone it before
entering the closure alongside `bins` and `peaks`:

```rust
let noise_floor = frame.noise_floor;
// ... inside paint closure:
let nf = noise_floor.clamp(DB_MIN, DB_MAX) as f64;
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 4 — Register panel + `spectrum` preset + overlay

**Files:** `src/config.rs`, `src/app.rs`, `src/ui/overlay.rs`

- [ ] **Add `spectrum` preset to `src/config.rs`** in `default_config()`, after the `monitoring` preset:

```rust
let spectrum = PresetConfig {
    panels: vec![
        PanelSpec { name: "header".into(),   position: Top,    height: Some(3),  width_pct: None },
        PanelSpec { name: "spectrum".into(),  position: Body,   height: None,     width_pct: None },
        PanelSpec { name: "log".into(),       position: Bottom, height: Some(5),  width_pct: None },
        PanelSpec { name: "footer".into(),    position: Bottom, height: Some(3),  width_pct: None },
    ],
};
presets.insert("spectrum".into(), spectrum);
```

- [ ] **Register `SpectrumPanel` in `src/app.rs`** alongside the other panel registrations:

```rust
registry.register(ui::SpectrumPanel);
```

- [ ] **Add `3` key handler** in the `InputMode::Normal` match arm in `src/app.rs`:

```rust
KeyCode::Char('3') => {
    self.engine.set_preset("spectrum");
    self.state.lock().unwrap().push_log("Preset: spectrum");
}
```

- [ ] **Update help overlay in `src/ui/overlay.rs`** — add after the `[2]` line:

```
 [3]        Preset: spectrum\n\
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 5 — Final validation

```bash
cargo build --release
cargo test
cargo clippy -- -D warnings
```

Expected: zero errors, zero warnings, all tests pass.

**Manual test checklist (HackRF connected):**

- [ ] Default preset (`minimal`) unchanged — Phase 7 panels still work
- [ ] Press `3` → switches to `spectrum` preset
  - [ ] "Waiting for RX…" shown in spectrum panel before streaming starts
- [ ] Press Space → RX starts
  - [ ] After ~2 s, spectrum bars appear (green bars, yellow peak-hold, gray noise floor)
  - [ ] Frequency axis labels update when frequency is changed with `f`
- [ ] Stop RX (Space again) → after 500 ms, `[STALE]` appears in title
- [ ] Press `1` → returns to minimal preset
- [ ] Press `p` → cycles: minimal → monitoring → spectrum → minimal
- [ ] All Phase 5 / 7 keys still work in all presets
