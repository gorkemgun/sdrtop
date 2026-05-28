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
        ColorDepth::Color256 | ColorDepth::Color16 => {
            magnitude_to_color(db, db_min, db_max, depth)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truecolor_cold_end_is_dark_blue() {
        let c = magnitude_to_color(-120.0, -120.0, 0.0, ColorDepth::TrueColor);
        assert_eq!(c, Color::Rgb(0, 0, 128));
    }

    #[test]
    fn truecolor_hot_end_is_red() {
        let c = magnitude_to_color(0.0, -120.0, 0.0, ColorDepth::TrueColor);
        assert_eq!(c, Color::Rgb(255, 0, 0));
    }

    #[test]
    fn clamp_below_min_same_as_min() {
        let at_min = magnitude_to_color(-120.0, -120.0, 0.0, ColorDepth::TrueColor);
        let below  = magnitude_to_color(-200.0, -120.0, 0.0, ColorDepth::TrueColor);
        assert_eq!(at_min, below, "values below db_min should clamp to cold end");
    }

    #[test]
    fn color16_covers_all_levels() {
        let cold = magnitude_to_color(-120.0, -120.0, 0.0, ColorDepth::Color16);
        let hot  = magnitude_to_color(   0.0, -120.0, 0.0, ColorDepth::Color16);
        assert_eq!(cold, Color::DarkGray);
        assert_eq!(hot,  Color::White);
    }

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
}
