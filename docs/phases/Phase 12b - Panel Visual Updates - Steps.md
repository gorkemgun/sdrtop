# Phase 12b — Panel Visual Updates: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)

**Goal:** Apply rounded borders + theme colors to every panel. Extend the spectrum
to render per-bin gradient bars using the theme palette. Wire the waterfall to the
theme palette as well. After this sub-phase the app looks visually transformed.

**Prerequisite:** [Phase 12a](Phase%2012a%20-%20Theme%20Foundation%20-%20Steps.md) complete.
`cargo test` passes, zero clippy warnings. `theme: &crate::Theme` reaches every
`panel.render()` call site with `_theme` as placeholder.

**Sub-phases:** [12a](Phase%2012a%20-%20Theme%20Foundation%20-%20Steps.md) → 12b → [12c](Phase%2012c%20-%20Header%20Footer%20Focus%20-%20Steps.md)

---

## Border role reference

Every panel must choose one of three border roles. Use this table throughout the steps:

| Panel | Border field | Rationale |
|---|---|---|
| `spectrum` | `border_accent` | primary visual — the thing you look at |
| `waterfall` | `border_accent` | primary visual |
| `rf_chain` | `border_default` | info panel |
| `hardware_health` | `border_default` | info panel |
| `signal_metrics` | `border_default` | info panel |
| `iq_diagnostics` | `border_default` | info panel |
| `iq_histogram` | `border_default` | info panel |
| `telemetry` | `border_default` | info panel |
| `gains` | `border_dim` | secondary — gain controls |
| `system_resources` | `border_dim` | secondary |
| `log` | `border_dim` | secondary |
| `observer` | `border_accent` | primary when in observer mode |
| `header` | *(redesigned in 12c)* | — |
| `footer` | *(redesigned in 12c)* | — |

---

## The pattern — apply it to every panel

For each panel, three changes:

1. **Border type + color:**
```rust
// BEFORE:
let block = Block::default()
    .title(" Panel Title ")
    .borders(Borders::ALL);

// AFTER:
let block = Block::default()
    .title(" Panel Title ")
    .borders(Borders::ALL)
    .border_type(ratatui::widgets::BorderType::Rounded)
    .border_style(Style::default().fg(theme.border_default)); // or border_accent / border_dim
```

2. **Labels → `theme.label`:**
```rust
// BEFORE:
let lbl = Style::default().fg(Color::DarkGray);

// AFTER:
let lbl = Style::default().fg(theme.label);
```

3. **Values → `theme.value` or `theme.value_hi`:**
```rust
// BEFORE:
let val = Style::default().fg(Color::White);
let hi  = Style::default().fg(Color::Cyan);

// AFTER:
let val = Style::default().fg(theme.value);
let hi  = Style::default().fg(theme.value_hi);
```

4. **Status colors → `theme.status_ok / status_warn / status_crit`:**
```rust
// BEFORE:
fn threshold_color(value: f64, warn: f64, crit: f64) -> Color {
    if value >= crit      { Color::Red    }
    else if value >= warn { Color::Yellow }
    else                  { Color::Green  }
}

// AFTER — pass theme:
fn threshold_color(value: f64, warn: f64, crit: f64, theme: &crate::Theme) -> Color {
    if value >= crit      { theme.status_crit }
    else if value >= warn { theme.status_warn }
    else                  { theme.status_ok   }
}
```

Change `_theme` to `theme` in the panel's `render` signature when you touch it.

---

## Step 1 — `hardware_health.rs`

**Files:** `src/ui/hardware_health.rs`

- [ ] **Update `render` signature** — remove the underscore:
```rust
fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme) {
```

- [ ] **Update `threshold_color`** — add `theme` param and replace Color constants:
```rust
fn threshold_color(value: f64, warn: f64, crit: f64, theme: &crate::Theme) -> Color {
    if value >= crit      { theme.status_crit }
    else if value >= warn { theme.status_warn }
    else                  { theme.status_ok   }
}
```

- [ ] **Update all `threshold_color` call sites** to pass `theme`:
```rust
let drop_color = threshold_color(state.drops_per_sec as f64, 1.0, 10.0, theme);
let sat_color  = threshold_color(state.adc_saturation_pct as f64, 1.0, 5.0, theme);
let jitter_color = threshold_color(state.callback_jitter_us as f64, 500.0, 2000.0, theme);
```

- [ ] **USB errors color**:
```rust
let usb_color = if state.usb_errors_session > 0 { theme.status_crit } else { theme.status_ok };
```

- [ ] **Update `Block`** with rounded border + `border_default`:
```rust
let block = Block::default()
    .title(" Hardware Health ")
    .borders(Borders::ALL)
    .border_type(ratatui::widgets::BorderType::Rounded)
    .border_style(Style::default().fg(theme.border_default));
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 2 — `rf_chain.rs`

**Files:** `src/ui/rf_chain.rs`

- [ ] **Update `render` signature.**

- [ ] **Update `Block`** — use `border_default`:
```rust
let block = Block::default()
    .title(" RF Chain ")
    .borders(Borders::ALL)
    .border_type(ratatui::widgets::BorderType::Rounded)
    .border_style(Style::default().fg(theme.border_default));
```

- [ ] **Replace label/value/hi styles**:
```rust
let lbl = Style::default().fg(theme.label);
let val = Style::default().fg(theme.value);
let hi  = Style::default().fg(theme.value_hi);
```

- [ ] **AMP ON color** — replace `Color::Yellow` with `theme.status_warn`:
```rust
if state.amp_enabled { Style::default().fg(theme.status_warn) } else { val }
```

- [ ] **CPLD status colors**:
```rust
Some(true)  => Span::styled("OK",       Style::default().fg(theme.status_ok)),
Some(false) => Span::styled("MISMATCH", Style::default().fg(theme.status_crit).add_modifier(Modifier::BOLD)),
None        => Span::styled("n/a",      Style::default().fg(theme.label)),
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 3 — `signal_metrics.rs`

**Files:** `src/ui/signal_metrics.rs`

- [ ] **Update `render` signature.**

- [ ] **`snr_color` function** — pass theme:
```rust
fn snr_color(snr: f32, theme: &crate::Theme) -> Color {
    if snr >= 20.0      { theme.status_ok   }
    else if snr >= 10.0 { theme.status_warn }
    else                { theme.status_crit }
}
```

- [ ] **Update `Block`** — use `border_default`; `stale` color when frame is stale:
```rust
let block = Block::default()
    .title(title)
    .borders(Borders::ALL)
    .border_type(ratatui::widgets::BorderType::Rounded)
    .border_style(Style::default().fg(
        if stale { theme.stale } else { theme.border_default }
    ));
```

- [ ] **Update label/value styles** and the SNR color call:
```rust
let lbl = Style::default().fg(theme.label);
let val = Style::default().fg(theme.value);
// ...
Span::styled(format!("{:.1} dB", state.snr_db), Style::default().fg(snr_color(state.snr_db, theme))),
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 4 — `iq_diagnostics.rs`, `iq_histogram.rs`, `gains.rs`, `telemetry.rs`

**Files:** `src/ui/iq_diagnostics.rs`, `src/ui/iq_histogram.rs`, `src/ui/gains.rs`, `src/ui/telemetry.rs`

Apply the same pattern to all four files. Key differences:

**`iq_diagnostics.rs`** — `border_default`:
```rust
.border_type(ratatui::widgets::BorderType::Rounded)
.border_style(Style::default().fg(theme.border_default))
```
Replace all `Color::DarkGray` labels with `theme.label`, `Color::White` values
with `theme.value`, `Color::Green/Yellow/Red` statuses with `theme.status_ok/warn/crit`.

**`iq_histogram.rs`** — `border_default`. Replace `Color::Red` (clipping) with
`theme.status_crit`, `Color::Yellow` (weak signal) with `theme.status_warn`,
`Color::Green` (OK) with `theme.status_ok`:
```rust
let note = if hist[28..32].iter().sum::<u64>() > max_count / 10 {
    Span::styled("▲ clipping risk", Style::default().fg(theme.status_crit))
} else if hist[0..8].iter().sum::<u64>() > hist.iter().sum::<u64>() * 9 / 10 {
    Span::styled("▼ weak signal",   Style::default().fg(theme.status_warn))
} else {
    Span::styled("dynamic range OK", Style::default().fg(theme.status_ok))
};
```

**`gains.rs`** — `border_dim`. This is a secondary panel:
```rust
.border_type(ratatui::widgets::BorderType::Rounded)
.border_style(Style::default().fg(theme.border_dim))
```

**`telemetry.rs`** — `border_default`, streaming status uses `status_ok`/`status_warn`:
```rust
let status_color = if m.hw_streaming { theme.status_ok } else { theme.status_warn };
```
The `Block` border color should reflect streaming status (this panel already did this
with `Color::Green`/`Color::Yellow` — preserve the behavior, just use theme colors).

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 5 — `system_resources.rs`, `log.rs`, `observer.rs`

**Files:** `src/ui/system_resources.rs`, `src/ui/log.rs`, `src/ui/observer.rs`

**`system_resources.rs`** — `border_dim`:
```rust
.border_type(ratatui::widgets::BorderType::Rounded)
.border_style(Style::default().fg(theme.border_dim))
```
Replace any `Color::*` label/value/status colors with theme equivalents.

**`log.rs`** — `border_dim`. The log panel is mostly text; just update the block:
```rust
.border_type(ratatui::widgets::BorderType::Rounded)
.border_style(Style::default().fg(theme.border_dim))
```

**`observer.rs`** — `border_accent` (observer is the primary panel when in observer mode):
```rust
.border_type(ratatui::widgets::BorderType::Rounded)
.border_style(Style::default().fg(theme.observer))
```
Replace the yellow `Color::Yellow` title border with `theme.observer`. Status values
(owner CPU%, RAM, uptime) use `theme.value`; labels use `theme.label`.

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 6 — `palette.rs`: extend to support theme gradient

**Files:** `src/palette.rs`

The existing `magnitude_to_color()` uses a hardcoded truecolor gradient. We need
a variant that accepts the theme's custom palette for the TrueColor case.

- [ ] **Add `magnitude_to_color_themed()`** — new function that takes the theme
  palette instead of using the hardcoded stops. Append after the existing
  `magnitude_to_color()`:

```rust
/// Like `magnitude_to_color` but uses the theme's custom gradient for TrueColor.
/// For Color256 and Color16 it falls back to the existing hardcoded palettes.
pub fn magnitude_to_color_themed(
    db: f32,
    db_min: f32,
    db_max: f32,
    depth: ColorDepth,
    theme: &crate::Theme,
) -> Color {
    let t = ((db - db_min) / (db_max - db_min)).clamp(0.0, 1.0);
    match depth {
        ColorDepth::TrueColor => theme.palette_color(t),
        // For 256-color and 16-color, fall back to existing palettes
        ColorDepth::Color256 | ColorDepth::Color16 => {
            magnitude_to_color(db, db_min, db_max, depth)
        }
    }
}
```

- [ ] **Add tests**:
```rust
    #[test]
    fn themed_truecolor_uses_theme_palette() {
        let theme = crate::Theme::sdr();
        let cold = magnitude_to_color_themed(-120.0, -120.0, 0.0, ColorDepth::TrueColor, &theme);
        let hot  = magnitude_to_color_themed(   0.0, -120.0, 0.0, ColorDepth::TrueColor, &theme);
        // SDR palette cold end is (10, 10, 80), hot end is (255, 50, 20)
        assert_eq!(cold, Color::Rgb(10, 10, 80));
        assert_eq!(hot,  Color::Rgb(255, 50, 20));
    }

    #[test]
    fn themed_256color_falls_back_to_existing() {
        let theme = crate::Theme::sdr();
        let result   = magnitude_to_color_themed(-60.0, -120.0, 0.0, ColorDepth::Color256, &theme);
        let existing = magnitude_to_color(       -60.0, -120.0, 0.0, ColorDepth::Color256);
        assert_eq!(result, existing);
    }
```

- [ ] **Run `cargo test palette::tests`**. Expected: all pass (4 existing + 2 new).

---

## Step 7 — `spectrum.rs`: per-bin gradient bars

**Files:** `src/ui/spectrum.rs`

The spectrum currently draws every bin with `Color::Green`. After this step each
bin's bar is colored by its dBFS value using the theme gradient.

- [ ] **Update `render` signature.**

- [ ] **Add `use crate::palette::{magnitude_to_color_themed, ColorDepth};`** at the top.

- [ ] **Detect color depth once** at render time — add at the start of the `Some(frame)` match arm:
```rust
let depth = crate::palette::ColorDepth::detect();
```

- [ ] **Update the spectrum Canvas paint closure** — change the bar color from
  `Color::Green` to a per-bin themed color. The closure captures `bins` and `theme`.
  Because `theme` is `&crate::Theme` (not `'static`), clone the palette into the
  closure:

```rust
let palette = theme.palette.clone();
let peak_hold_color = theme.peak_hold;
let noise_floor_color = theme.noise_floor;
let stale_color = theme.stale;
```

Then in the paint closure:
```rust
.paint(move |ctx| {
    // Build a temporary Theme-like struct just for palette_color
    // Simpler: precompute colors outside the closure for each bin
    // We can't call theme.palette_color() inside the closure because
    // closures can't capture &Theme with a non-static lifetime in Canvas.
    // Solution: precompute a Vec<Color> outside the closure.
    ...
})
```

**The closure lifetime issue:** `Canvas::paint` requires a `'static` closure on
ratatui 0.26 (or at least `FnOnce` without captured references). The solution is to
precompute the per-bin colors before entering the closure:

```rust
// Precompute colors for each bin (outside the closure)
let bin_colors: Vec<Color> = bins.iter()
    .map(|&db| {
        let t = ((db - DB_MIN) / (DB_MAX - DB_MIN)).clamp(0.0, 1.0);
        crate::palette::magnitude_to_color_themed(db, DB_MIN, DB_MAX, depth, theme)
    })
    .collect();
let peak_hold_color = theme.peak_hold;
let noise_floor_color = theme.noise_floor;
let bins_clone = bins.clone();
let peaks_clone = peaks.clone();
```

Then in the paint closure (which captures by move, owning the precomputed data):
```rust
.paint(move |ctx| {
    // Spectrum bars — each bin colored by its dBFS value
    for (i, (&db, &color)) in bins_clone.iter().zip(bin_colors.iter()).enumerate() {
        let y = db.clamp(DB_MIN, DB_MAX) as f64;
        ctx.draw(&CanvasLine {
            x1: i as f64, y1: DB_MIN as f64,
            x2: i as f64, y2: y,
            color,
        });
    }
    // Peak hold
    for (i, &db) in peaks_clone.iter().enumerate() {
        let y = db.clamp(DB_MIN, DB_MAX) as f64;
        ctx.draw(&Points {
            coords: &[(i as f64, y)],
            color: peak_hold_color,
        });
    }
    // Noise floor line
    let nf = noise_floor.clamp(DB_MIN, DB_MAX) as f64;
    ctx.draw(&CanvasLine {
        x1: 0.0, y1: nf,
        x2: n,   y2: nf,
        color: noise_floor_color,
    });
})
```

- [ ] **Update the `[STALE]` border** — when stale, use `theme.stale`:
```rust
.border_style(if stale {
    Style::default().fg(theme.stale)
} else {
    Style::default().fg(theme.border_accent)
})
```

- [ ] **Update the "Waiting for RX" placeholder** to use `theme.label`:
```rust
.style(Style::default().fg(theme.label))
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 8 — `waterfall.rs`: theme palette

**Files:** `src/ui/waterfall.rs`

- [ ] **Update `render` signature.**

- [ ] **Remove `self.color_depth` field** from `WaterfallPanel` — we'll detect at
  render time instead (removes the need to store it). Update `WaterfallPanel::new()`:

```rust
pub struct WaterfallPanel;

impl WaterfallPanel {
    pub fn new() -> Self { Self }
}
```

- [ ] **Use `magnitude_to_color_themed`** inside the cell loop. Add the import:
```rust
use crate::palette::{magnitude_to_color_themed, ColorDepth};
```

In `render`, detect depth and use themed color:
```rust
let depth = ColorDepth::detect();
// ...inside the cell loop:
let color = magnitude_to_color_themed(db, DB_MIN, DB_MAX, depth, theme);
```

- [ ] **Update the `[PAUSED]` and stale border** — use `theme.stale` when paused:
```rust
let block = Block::default()
    .title(title)
    .borders(Borders::ALL)
    .border_type(ratatui::widgets::BorderType::Rounded)
    .border_style(Style::default().fg(
        if buf.paused { theme.stale } else { theme.border_accent }
    ));
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Run `cargo test`**. Expected: all tests pass.

- [ ] **Run `cargo clippy -- -D warnings`**. Expected: zero warnings.

---

## Final visual validation

Run the app and switch through all presets:

```bash
cargo run
```

- [ ] Press `1` → minimal: all panels have rounded corners and themed colors
- [ ] Press `2` → monitoring: `HardwareHealthPanel` shows theme-colored threshold indicators
- [ ] Press `3` → spectrum: spectrum bars show color gradient (cold = theme cold, hot = theme hot)
- [ ] Press `4` → waterfall: rows colored by theme palette
- [ ] Press `5` → spectrum + waterfall: both panels match theme palette
- [ ] Press `6` → lab: `RfChainPanel` shows `value_hi` for frequency and total gain;
  `IqHistogramPanel` shows status colors from theme
- [ ] Edit `~/.config/sdrtop/config.toml`, add `[theme]\nbase = "nord"` and restart —
  the whole app shifts to a blue-gray arctic palette
- [ ] Try `base = "gruvbox"` — warm amber/olive tones
- [ ] Try `base = "dracula"` — purple accent
