# Phase 12a — Theme Foundation: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md) 

**Goal:** Build the `Theme` struct with 6 built-in palettes, wire it into `AppConfig`,
update the `Panel` trait signature, and add `board_name`/`serial` to `SdrMetrics`.
After this sub-phase the codebase compiles with the new trait; no visual change yet.

**Prerequisite:** Phase 11 complete. `cargo test` passes, zero clippy warnings.

**Sub-phases:** 12a → [12b](Phase%2012b%20-%20Panel%20Visual%20Updates%20-%20Steps.md) → [12c](Phase%2012c%20-%20Header%20Footer%20Focus%20-%20Steps.md)

---

## Dependency order

```
src/theme.rs            new: Theme struct + 6 built-in themes + tests
    ↓
src/config.rs           new: ThemeConfig + hex parse + build_theme()
    ↓
src/ui/panel.rs         updated: Panel::render gains theme: &Theme param
                        new: focus_key() + focus_bindings() default methods
    ↓
src/state.rs            new: board_name: String, serial: String on SdrMetrics
    ↓
src/app.rs              updated: populate board_name/serial; pass theme in draw loop
```

---

## Step 1 — `src/theme.rs`: Theme struct + 6 built-in themes

**Files:** `src/theme.rs` (new)

- [ ] **Create `src/theme.rs`:**

```rust
use ratatui::style::Color;

/// All colors used anywhere in the UI. No panel file hardcodes a Color after Phase 12.
#[derive(Clone)]
pub struct Theme {
    pub name: &'static str,

    // Borders — three tiers of visual weight
    pub border_dim: Color,      // log, system_resources, gains (background panels)
    pub border_default: Color,  // rf_chain, hardware_health, signal_metrics, iq_*
    pub border_accent: Color,   // spectrum, waterfall (primary visual panels)
    pub border_focused: Color,  // any panel currently in panel-focus mode

    // Text
    pub label: Color,           // dim labels: "Frequency", "LNA gain"
    pub value: Color,           // normal values
    pub value_hi: Color,        // highlighted values: frequency, total gain, board name

    // Status indicators
    pub status_ok: Color,
    pub status_warn: Color,
    pub status_crit: Color,

    // Spectrum & waterfall gradient. Each stop: (t ∈ [0,1], r, g, b).
    // Cold (t=0) is weak signal; hot (t=1) is strong signal.
    pub palette: Vec<(f32, u8, u8, u8)>,
    pub peak_hold: Color,
    pub noise_floor: Color,

    // Misc
    pub stale: Color,      // [STALE] title + dim border when FFT frame is old
    pub observer: Color,   // observer mode status dot + accent
}

fn rgb(r: u8, g: u8, b: u8) -> Color { Color::Rgb(r, g, b) }

impl Theme {
    /// Default theme. Designed specifically for sdrtop: deep black bg, sharp cyan accent,
    /// orange highlighted values, lime-to-red spectrum gradient.
    pub fn sdr() -> Self {
        Self {
            name: "sdr",
            border_dim:     rgb(45, 50, 65),
            border_default: rgb(60, 120, 145),
            border_accent:  rgb(0, 215, 255),
            border_focused: rgb(255, 255, 255),
            label:          rgb(90, 100, 115),
            value:          rgb(195, 210, 220),
            value_hi:       rgb(255, 175, 0),
            status_ok:      rgb(0, 210, 130),
            status_warn:    rgb(255, 175, 0),
            status_crit:    rgb(255, 65, 65),
            palette: vec![
                (0.00,  10,  10,  80),
                (0.25,   0,  80, 180),
                (0.45,   0, 210, 210),
                (0.60,   0, 210,  80),
                (0.78, 255, 215,   0),
                (1.00, 255,  50,  20),
            ],
            peak_hold:   rgb(255, 215, 0),
            noise_floor: rgb(55, 65, 80),
            stale:       rgb(60, 65, 75),
            observer:    rgb(100, 150, 255),
        }
    }

    /// Arctic Professional. Nord color system (https://www.nordtheme.com/).
    pub fn nord() -> Self {
        Self {
            name: "nord",
            border_dim:     rgb(59, 66, 82),
            border_default: rgb(76, 86, 106),
            border_accent:  rgb(136, 192, 208),
            border_focused: rgb(216, 222, 233),
            label:          rgb(76, 86, 106),
            value:          rgb(216, 222, 233),
            value_hi:       rgb(235, 203, 139),
            status_ok:      rgb(163, 190, 140),
            status_warn:    rgb(235, 203, 139),
            status_crit:    rgb(191, 97, 106),
            palette: vec![
                (0.00,  36,  41,  54),
                (0.25,  67, 103, 141),
                (0.50, 136, 192, 208),
                (0.65, 163, 190, 140),
                (0.82, 235, 203, 139),
                (1.00, 191,  97, 106),
            ],
            peak_hold:   rgb(235, 203, 139),
            noise_floor: rgb(59, 66, 82),
            stale:       rgb(59, 66, 82),
            observer:    rgb(129, 161, 193),
        }
    }

    /// Dracula (https://draculatheme.com/).
    pub fn dracula() -> Self {
        Self {
            name: "dracula",
            border_dim:     rgb(68, 71, 90),
            border_default: rgb(98, 114, 164),
            border_accent:  rgb(189, 147, 249),
            border_focused: rgb(248, 248, 242),
            label:          rgb(98, 114, 164),
            value:          rgb(248, 248, 242),
            value_hi:       rgb(241, 250, 140),
            status_ok:      rgb(80, 250, 123),
            status_warn:    rgb(255, 184, 108),
            status_crit:    rgb(255, 85, 85),
            palette: vec![
                (0.00,  40,  42,  54),
                (0.25,  98, 114, 164),
                (0.48, 139, 233, 253),
                (0.63,  80, 250, 123),
                (0.80, 255, 184, 108),
                (1.00, 255,  85,  85),
            ],
            peak_hold:   rgb(241, 250, 140),
            noise_floor: rgb(68, 71, 90),
            stale:       rgb(68, 71, 90),
            observer:    rgb(139, 233, 253),
        }
    }

    /// Gruvbox Dark (https://github.com/morhetz/gruvbox).
    pub fn gruvbox() -> Self {
        Self {
            name: "gruvbox",
            border_dim:     rgb(60, 56, 54),
            border_default: rgb(102, 92, 84),
            border_accent:  rgb(215, 153, 33),
            border_focused: rgb(235, 219, 178),
            label:          rgb(102, 92, 84),
            value:          rgb(213, 196, 161),
            value_hi:       rgb(250, 189, 47),
            status_ok:      rgb(152, 151, 26),
            status_warn:    rgb(215, 153, 33),
            status_crit:    rgb(204, 36, 29),
            palette: vec![
                (0.00,  40,  40,  40),
                (0.25,  69, 133, 136),
                (0.48, 152, 151,  26),
                (0.63, 215, 153,  33),
                (0.80, 214,  93,  14),
                (1.00, 204,  36,  29),
            ],
            peak_hold:   rgb(250, 189, 47),
            noise_floor: rgb(60, 56, 54),
            stale:       rgb(60, 56, 54),
            observer:    rgb(69, 133, 136),
        }
    }

    /// Catppuccin Mocha (https://catppuccin.com/).
    pub fn catppuccin() -> Self {
        Self {
            name: "catppuccin",
            border_dim:     rgb(49, 50, 68),
            border_default: rgb(88, 91, 112),
            border_accent:  rgb(203, 166, 247),
            border_focused: rgb(205, 214, 244),
            label:          rgb(88, 91, 112),
            value:          rgb(205, 214, 244),
            value_hi:       rgb(249, 226, 175),
            status_ok:      rgb(166, 227, 161),
            status_warn:    rgb(249, 226, 175),
            status_crit:    rgb(243, 139, 168),
            palette: vec![
                (0.00,  30,  30,  46),
                (0.25, 116, 199, 236),
                (0.48, 137, 220, 235),
                (0.63, 166, 227, 161),
                (0.80, 249, 226, 175),
                (1.00, 243, 139, 168),
            ],
            peak_hold:   rgb(249, 226, 175),
            noise_floor: rgb(49, 50, 68),
            stale:       rgb(49, 50, 68),
            observer:    rgb(137, 220, 235),
        }
    }

    /// Solarized Dark (https://ethanschoonover.com/solarized/).
    pub fn solarized() -> Self {
        Self {
            name: "solarized",
            border_dim:     rgb(7, 54, 66),
            border_default: rgb(88, 110, 117),
            border_accent:  rgb(38, 139, 210),
            border_focused: rgb(253, 246, 227),
            label:          rgb(88, 110, 117),
            value:          rgb(147, 161, 161),
            value_hi:       rgb(181, 137, 0),
            status_ok:      rgb(133, 153, 0),
            status_warn:    rgb(181, 137, 0),
            status_crit:    rgb(220, 50, 47),
            palette: vec![
                (0.00,   0,  43,  54),
                (0.25,  38, 139, 210),
                (0.48,  42, 161, 152),
                (0.63, 133, 153,   0),
                (0.80, 181, 137,   0),
                (1.00, 220,  50,  47),
            ],
            peak_hold:   rgb(181, 137, 0),
            noise_floor: rgb(7, 54, 66),
            stale:       rgb(7, 54, 66),
            observer:    rgb(42, 161, 152),
        }
    }

    /// Return a built-in theme by name. Unknown name → `sdr` (default).
    pub fn by_name(name: &str) -> Self {
        match name {
            "nord"       => Self::nord(),
            "dracula"    => Self::dracula(),
            "gruvbox"    => Self::gruvbox(),
            "catppuccin" => Self::catppuccin(),
            "solarized"  => Self::solarized(),
            _            => Self::sdr(),
        }
    }

    /// Parse a "#rrggbb" hex string into `Color::Rgb`. Returns `None` on invalid input.
    pub fn parse_hex(s: &str) -> Option<Color> {
        let s = s.trim().strip_prefix('#')?;
        if s.len() != 6 { return None; }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    }

    /// Interpolate within the theme's gradient palette. `t` ∈ [0.0, 1.0].
    pub fn palette_color(&self, t: f32) -> Color {
        if self.palette.is_empty() { return Color::White; }
        let t = t.clamp(0.0, 1.0);
        for i in 0..self.palette.len().saturating_sub(1) {
            let (t0, r0, g0, b0) = self.palette[i];
            let (t1, r1, g1, b1) = self.palette[i + 1];
            if t <= t1 {
                let s = if (t1 - t0).abs() < f32::EPSILON { 0.0 } else { (t - t0) / (t1 - t0) };
                let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * s) as u8;
                return Color::Rgb(lerp(r0, r1), lerp(g0, g1), lerp(b0, b1));
            }
        }
        let (_, r, g, b) = *self.palette.last().unwrap();
        Color::Rgb(r, g, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn by_name_unknown_falls_back_to_sdr() {
        let t = Theme::by_name("does_not_exist");
        assert_eq!(t.name, "sdr");
    }

    #[test]
    fn by_name_returns_correct_theme() {
        assert_eq!(Theme::by_name("nord").name, "nord");
        assert_eq!(Theme::by_name("dracula").name, "dracula");
        assert_eq!(Theme::by_name("gruvbox").name, "gruvbox");
        assert_eq!(Theme::by_name("catppuccin").name, "catppuccin");
        assert_eq!(Theme::by_name("solarized").name, "solarized");
    }

    #[test]
    fn all_themes_have_non_empty_palette() {
        for name in &["sdr", "nord", "dracula", "gruvbox", "catppuccin", "solarized"] {
            let t = Theme::by_name(name);
            assert!(!t.palette.is_empty(), "theme '{}' has empty palette", name);
        }
    }

    #[test]
    fn parse_hex_valid_colors() {
        assert_eq!(Theme::parse_hex("#00d7ff"), Some(Color::Rgb(0, 215, 255)));
        assert_eq!(Theme::parse_hex("#88c0d0"), Some(Color::Rgb(136, 192, 208)));
        assert_eq!(Theme::parse_hex("#000000"), Some(Color::Rgb(0, 0, 0)));
        assert_eq!(Theme::parse_hex("#ffffff"), Some(Color::Rgb(255, 255, 255)));
    }

    #[test]
    fn parse_hex_invalid_returns_none() {
        assert_eq!(Theme::parse_hex("00d7ff"), None);  // missing #
        assert_eq!(Theme::parse_hex("#gggggg"), None); // invalid hex chars
        assert_eq!(Theme::parse_hex("#fff"), None);    // too short
        assert_eq!(Theme::parse_hex(""), None);
    }

    #[test]
    fn palette_color_cold_end() {
        let t = Theme::sdr();
        let c = t.palette_color(0.0);
        assert_eq!(c, Color::Rgb(10, 10, 80));
    }

    #[test]
    fn palette_color_hot_end() {
        let t = Theme::sdr();
        let c = t.palette_color(1.0);
        assert_eq!(c, Color::Rgb(255, 50, 20));
    }

    #[test]
    fn palette_color_clamps_out_of_range() {
        let t = Theme::sdr();
        assert_eq!(t.palette_color(-1.0), t.palette_color(0.0));
        assert_eq!(t.palette_color(2.0), t.palette_color(1.0));
    }
}
```

- [ ] **Add `mod theme;` and `pub use theme::Theme;`** to `src/main.rs` or wherever the crate root is. In this project the crate root is `src/main.rs`. Add before the existing `mod app;`:

```rust
mod theme;
pub use theme::Theme;
```

- [ ] **Run `cargo test theme::tests`**. Expected: all 9 tests pass.

---

## Step 2 — `src/config.rs`: ThemeConfig + `build_theme()`

**Files:** `src/config.rs`

The existing `AppConfig` has `RadioConfig` and `DisplayConfig` under `[radio]` and `[display]`. Add a `[theme]` section.

- [ ] **Add `ThemeConfig`** — insert after the `DisplayConfig` struct definition:

```rust
#[derive(Debug, Deserialize, Default)]
pub struct ThemeConfig {
    #[serde(default = "ThemeConfig::default_base")]
    pub base: String,
    // Per-field overrides. "#rrggbb" strings. None = use theme default.
    pub border_accent:  Option<String>,
    pub border_dim:     Option<String>,
    pub border_default: Option<String>,
    pub border_focused: Option<String>,
    pub label:          Option<String>,
    pub value:          Option<String>,
    pub value_hi:       Option<String>,
    pub status_ok:      Option<String>,
    pub status_warn:    Option<String>,
    pub status_crit:    Option<String>,
    pub peak_hold:      Option<String>,
    pub noise_floor:    Option<String>,
    pub stale:          Option<String>,
    pub observer:       Option<String>,
}

impl ThemeConfig {
    fn default_base() -> String { "sdr".into() }
}
```

- [ ] **Add `theme: ThemeConfig` field** to `AppConfig`:

```rust
pub struct AppConfig {
    pub radio:   RadioConfig,
    pub display: DisplayConfig,
    #[serde(default)]
    pub theme:   ThemeConfig,
}
```

- [ ] **Add `build_theme()` method to `AppConfig`** — this constructs the final `Theme` by starting from the named base and applying any per-field overrides. Add at the end of `impl AppConfig`:

```rust
    pub fn build_theme(&self) -> crate::Theme {
        let mut t = crate::Theme::by_name(&self.theme.base);
        let tc = &self.theme;
        // Apply each override if present and parseable
        macro_rules! apply {
            ($field:ident) => {
                if let Some(ref s) = tc.$field {
                    if let Some(c) = crate::Theme::parse_hex(s) {
                        t.$field = c;
                    }
                }
            };
        }
        apply!(border_accent);
        apply!(border_dim);
        apply!(border_default);
        apply!(border_focused);
        apply!(label);
        apply!(value);
        apply!(value_hi);
        apply!(status_ok);
        apply!(status_warn);
        apply!(status_crit);
        apply!(peak_hold);
        apply!(noise_floor);
        apply!(stale);
        apply!(observer);
        t
    }
```

- [ ] **Add tests** — add to the existing `#[cfg(test)] mod tests` block in `src/config.rs`:

```rust
    #[test]
    fn build_theme_default_is_sdr() {
        let cfg = AppConfig::load_or_default(None);
        let t = cfg.build_theme();
        assert_eq!(t.name, "sdr");
    }

    #[test]
    fn build_theme_unknown_base_falls_back_to_sdr() {
        let toml = "[theme]\nbase = \"nonexistent\"\n";
        let cfg: AppConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.build_theme().name, "sdr");
    }

    #[test]
    fn build_theme_override_applies_hex_color() {
        let toml = "[theme]\nbase = \"nord\"\nborder_accent = \"#ff0000\"\n";
        let cfg: AppConfig = toml::from_str(toml).unwrap();
        let t = cfg.build_theme();
        assert_eq!(t.border_accent, ratatui::style::Color::Rgb(255, 0, 0));
    }

    #[test]
    fn build_theme_invalid_hex_override_ignored() {
        let toml = "[theme]\nbase = \"nord\"\nborder_accent = \"notahex\"\n";
        let cfg: AppConfig = toml::from_str(toml).unwrap();
        // Should keep nord's default border_accent, not crash
        let t = cfg.build_theme();
        assert_eq!(t.name, "nord");
    }
```

- [ ] **Run `cargo test config::tests`**. Expected: all existing + 4 new tests pass.

---

## Step 3 — `src/ui/panel.rs`: new trait signature + focus methods

**Files:** `src/ui/panel.rs`

The current `Panel` trait:

```rust
pub trait Panel: Send + Sync {
    fn name(&self) -> &'static str;
    fn min_size(&self) -> (u16, u16);
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics);
}
```

- [ ] **Update `Panel` trait** — replace the entire trait definition with:

```rust
pub trait Panel: Send + Sync {
    fn name(&self) -> &'static str;
    fn min_size(&self) -> (u16, u16);
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme);

    /// Single character that activates panel-focus mode for this panel.
    /// Returns `None` for panels that don't support focus mode.
    fn focus_key(&self) -> Option<char> { None }

    /// Keybindings shown in the footer when this panel is focused.
    /// Each entry: (key_label, description). Empty by default.
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] { &[] }
}
```

- [ ] **Fix the compile error** — every `impl Panel for XPanel` now has a mismatched
  `render` signature. The compiler will list all of them. For each panel, change:

```rust
fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
```

to:

```rust
fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, _theme: &crate::Theme) {
```

Use `_theme` (with underscore) for now — panels will use the theme in Phase 12b.
The panels to update are: `HeaderPanel`, `FooterPanel`, `TelemetryPanel`,
`GainsPanel`, `LogPanel`, `HardwareHealthPanel`, `IqDiagnosticsPanel`,
`SystemResourcesPanel`, `SpectrumPanel`, `WaterfallPanel`, `RfChainPanel`,
`SignalMetricsPanel`, `IqHistogramPanel`, `ObserverPanel`.

- [ ] **Run `cargo build`**. Expected: `Finished` with no errors. (Warnings about
  `_theme` being unused are fine for now.)

---

## Step 4 — `src/state.rs`: `board_name` + `serial` on `SdrMetrics`

**Files:** `src/state.rs`

Currently `board_name` and `serial` live as fields on `HeaderPanel` and
`TelemetryPanel`. After this step they live in `SdrMetrics` so any panel
(including the redesigned header) can read them without needing constructor params.

- [ ] **Add two fields** to `SdrMetrics` — insert near the other hardware-identity
  fields (`board_rev`, `usb_api_version`, `cpld_ok`):

```rust
    pub board_name: String,
    pub serial: String,
```

- [ ] **Initialize them** in the `SdrMetrics { ... }` literal in `src/app.rs`
  (there will be a compile error pointing at the exact location). Temporary
  empty strings — `App::new()` will populate them in Step 5:

```rust
    board_name: String::new(),
    serial: String::new(),
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 5 — `src/app.rs`: populate fields + pass theme to panels

**Files:** `src/app.rs`

**5a — Populate `board_name` and `serial` in `App::new_normal()`**

Find the block in `App::new_normal()` that reads device info at startup and logs
it. Currently the board name string and serial are stored in local variables
(used to construct `HeaderPanel` and `TelemetryPanel`). Move them into
`SdrMetrics` instead.

- [ ] After reading the board name and serial (the exact variable names depend on
  what's already there — look for the `let board_name = ...` and `let serial = ...`
  lines), populate the metrics:

```rust
        {
            let mut m = state.lock().unwrap();
            m.board_name = board_name.clone();
            m.serial     = serial.clone();
        }
```

**5b — Build theme from config and pass it into the draw loop**

The `App` struct needs to hold a `Theme`. Find the `App` struct definition and add:

```rust
    pub theme: crate::Theme,
```

In `App::new_normal()` (and `App::new_observer()` if it exists), build the theme
after loading config:

```rust
        let theme = config.build_theme();
```

And store it: `theme,` in the `App { ... }` struct literal.

- [ ] **Pass `&self.theme` to every `panel.render()` call** in the draw method.
  Find all sites where panels are rendered — typically in `app.rs` or `ui/mod.rs`
  in a loop like:

```rust
// BEFORE:
panel.render(f, area, &state);

// AFTER:
panel.render(f, area, &state, &self.theme);
```

If the draw logic is in `ui/engine.rs` or `ui/mod.rs` and doesn't have access to
`self.theme`, pass the theme as a parameter to the draw function. For example:

```rust
// In app.rs draw call:
engine.draw(f, &panels, &state, &self.theme);

// In engine.rs draw function signature:
pub fn draw(&self, f: &mut Frame, panels: &PanelRegistry, state: &SdrMetrics, theme: &crate::Theme) {
    // ...
    panel.render(f, area, state, theme);
}
```

Adapt to the actual structure — the key requirement is that `theme` reaches every
`panel.render()` call site.

- [ ] **Run `cargo build`**. Expected: `Finished` with no errors.

- [ ] **Run `cargo test`**. Expected: all tests pass.

- [ ] **Run `cargo clippy -- -D warnings`**. Expected: zero warnings.

---

## Step 6 — Update `save_config()` to persist theme

**Files:** `src/config.rs`, `src/app.rs`

The `save_config()` function currently writes `[radio]` and `[display]`. Since
the theme is chosen by the user, it should survive restarts.

- [ ] **Add theme serialization** to `AppConfig` — ensure `ThemeConfig` derives
  `Serialize` as well as `Deserialize`:

```rust
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ThemeConfig { ... }
```

- [ ] **Populate `ThemeConfig`** when building the save struct in `save_config()`.
  At minimum, write the `base` name so the chosen theme persists:

```rust
    theme: ThemeConfig {
        base: self.theme.name.to_string(),
        ..Default::default()
    },
```

- [ ] **Run `cargo build`** and **`cargo test`**. Expected: all pass.

---

## Step 7 — Update Roadmap + Home links

**Files:** `docs/Roadmap.md`, `docs/Home.md`

- [ ] **Update Roadmap Phase 12 steps links** — the Roadmap already has `**12.1**`
  through `**12.10**` as a flat list. Add a note at the top of the Phase 12 section:

```markdown
→ [Design spec](superpowers/specs/2026-05-28-ui-ux-polish-design.md)
→ [12a Steps (Theme Foundation)](phases/Phase%2012a%20-%20Theme%20Foundation%20-%20Steps.md)
→ [12b Steps (Panel Visual Updates)](phases/Phase%2012b%20-%20Panel%20Visual%20Updates%20-%20Steps.md)
→ [12c Steps (Header, Footer & Focus)](phases/Phase%2012c%20-%20Header%20Footer%20Focus%20-%20Steps.md)
```

- [ ] **Add the steps links to `docs/Home.md`** navigation table — after the Phase 12
  design spec link already there:

```markdown
| [Phase 12a - Theme Foundation - Steps](phases/Phase%2012a%20-%20Theme%20Foundation%20-%20Steps.md) | Phase 12a — Theme struct, config, Panel trait, SdrMetrics fields |
| [Phase 12b - Panel Visual Updates - Steps](phases/Phase%2012b%20-%20Panel%20Visual%20Updates%20-%20Steps.md) | Phase 12b — rounded borders, theme colors, spectrum gradient |
| [Phase 12c - Header Footer Focus - Steps](phases/Phase%2012c%20-%20Header%20Footer%20Focus%20-%20Steps.md) | Phase 12c — header redesign, footer redesign, panel focus system |
```

- [ ] **Run `cargo build && cargo test && cargo clippy -- -D warnings`**.
  Expected: clean.

---

## Final checklist before starting Phase 12b

- [ ] `cargo test` — all pass (9 theme tests + 4 config tests + all existing)
- [ ] `cargo clippy -- -D warnings` — zero warnings
- [ ] `cargo build --release` — clean
- [ ] App still runs and looks exactly like before (no visual change yet — theme is
  loaded and passed but panels still use `_theme`, ignoring it)
