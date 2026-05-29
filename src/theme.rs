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
            label:          rgb(145, 160, 180),
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
            noise_floor: rgb(80, 95, 120),
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
