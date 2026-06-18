use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::chrome;
use super::panel::Panel;

pub struct HeaderPanel;

/// A "breathing" RX status dot that cycles small→large→small on a ~0.9 s loop.
/// Pure glyph animation — the badge colours never change. All four glyphs are a
/// single terminal column, so the badge width (and the header gap math) is fixed.
/// Only animates while frames are flowing (RX), which is exactly when the UI
/// is already redrawing, so it costs no extra wakeups.
fn rx_pulse_glyph() -> &'static str {
    use std::time::{SystemTime, UNIX_EPOCH};
    const FRAMES: [&str; 4] = ["\u{2219}", "\u{2022}", "\u{25CF}", "\u{2022}"]; // ∙ • ● •
    let ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    FRAMES[((ms / 220) % FRAMES.len() as u128) as usize]
}

/// A dot-leader span that fills `gap` columns: ` ······ ` (one space at each
/// end, dim dots between), connecting a left field to a right one like an
/// engraved instrument readout. Falls back to plain spaces when too short.
fn leader(gap: usize, color: ratatui::style::Color) -> Span<'static> {
    if gap >= 4 {
        Span::styled(format!(" {} ", "·".repeat(gap - 2)), Style::default().fg(color))
    } else {
        Span::raw(" ".repeat(gap))
    }
}

/// Returns (filled_str, empty_str). Each string is exactly `n` terminal columns.
/// Uses the same segmented glyphs as the signal-strip gauges: ▮ filled, ▯ empty.
fn gain_bar(gain: u32, max_gain: u32, n: usize) -> (String, String) {
    let filled = ((gain as f32 / max_gain as f32) * n as f32).round() as usize;
    let filled = filled.min(n);
    ("▮".repeat(filled), "▯".repeat(n - filled))
}

/// Power-of-ten exponent (in Hz) of the digit the current tuning step acts on:
/// 1 kHz→3, 10 kHz→4, 100 kHz→5, 1 MHz→6, 10 MHz→7. Coarse-but-non-decade steps
/// (5 kHz, 25 kHz, 500 kHz, 5 MHz) collapse onto their leading digit's place,
/// which is the digit a user reads as "the one I'm moving".
fn step_place_exp(step_hz: u64) -> u32 {
    let mut e = 0u32;
    let mut s = step_hz.max(1);
    while s >= 10 { s /= 10; e += 1; }
    e
}

/// Segmented VFO frequency readout: the MHz value rendered digit-by-digit with a
/// thin gap between every character, and the single digit the current tuning step
/// moves underlined + brightened — so you can see at a glance which place `← →`
/// will change. The decimal point is dimmed. Returns the spans; width varies with
/// the number of MHz digits (the caller measures it for layout).
fn vfo_spans(freq_hz: u64, step_hz: u64, digit: ratatui::style::Color,
             dot: ratatui::style::Color, active: ratatui::style::Color) -> Vec<Span<'static>> {
    let s = format!("{:.3}", freq_hz as f64 / 1_000_000.0); // e.g. "145.500"
    let dot_pos = s.find('.').unwrap_or(s.len());
    let exp = step_place_exp(step_hz);

    // Char index (in `s`) of the active digit, if it is currently on screen.
    let active_idx: Option<usize> = if exp >= 6 {
        let from_right = (exp - 6) as usize;        // 0 = ones-MHz digit (just left of '.')
        (from_right < dot_pos).then(|| dot_pos - 1 - from_right)
    } else if (3..=5).contains(&exp) {
        Some(dot_pos + 1 + (5 - exp) as usize)       // 5→.1xx, 4→..1x, 3→...1
    } else {
        None
    };

    let chars: Vec<char> = s.chars().collect();
    let mut spans = Vec::with_capacity(chars.len() * 2);
    for (i, c) in chars.iter().enumerate() {
        if i > 0 { spans.push(Span::raw(" ")); }
        let style = if Some(i) == active_idx {
            Style::default().fg(active).add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else if *c == '.' {
            Style::default().fg(dot)
        } else {
            Style::default().fg(digit).add_modifier(Modifier::BOLD)
        };
        spans.push(Span::styled(c.to_string(), style));
    }
    spans
}

/// Returns the number of space characters needed between the fw-version field
/// and the right-aligned "AMP … USB …" section in the top band.
/// All length arguments are in terminal columns (chars, not bytes).
fn top_band_gap(board_name_len: usize, badge_len: usize, fw_value_len: usize,
                amp_val_len: usize, usb_val_len: usize, inner_width: u16) -> usize {
    // left side: " " + " DeviceName " + "  " + " BADGE " + "  " + "hackrf fw " + fw_val
    let left  = 1 + (2 + board_name_len) + 2 + badge_len + 2 + 10 + fw_value_len;
    // right side: "AMP "(4) + amp_val + "  ·  "(5) + "USB "(4) + usb_val + "  "(2)
    let right = 4 + amp_val_len + 5 + 4 + usb_val_len + 2;
    (inner_width as usize).saturating_sub(left + right)
}

fn top_band_line(state: &SdrMetrics, theme: &crate::Theme, inner_width: u16) -> Line<'static> {
    use ratatui::style::Color;

    // --- Status badge ---
    // RX uses a breathing dot; IDLE/OBSERVER are steady. Every variant is 6
    // columns so `top_band_gap` stays valid.
    let (badge_text, badge_bg, badge_fg): (String, Color, Color) = if state.observer.active {
        (" ◈ OBSERVER ".to_string(), theme.observer, Color::Rgb(4, 6, 15))
    } else if state.radio.hw_streaming {
        (format!(" {} RX ", rx_pulse_glyph()), theme.status_ok, Color::Rgb(3, 15, 6))
    } else {
        (" ○ IDLE ".to_string(), theme.status_warn, Color::Rgb(10, 7, 0))
    };
    let badge_len = badge_text.chars().count();

    // --- Firmware version + label ---
    // Mayhem nightly: "n_XXXXXX"; Mayhem release: "vX.Y.Z" → label as "mayhem fw "
    // Standard HackRF firmware ("2024.02.1", "git-...") → label as "hackrf fw "
    // Both labels are exactly 10 chars so top_band_gap stays valid.
    // Firmware field. RTL-SDR has no on-device firmware (it's host-driven by
    // librtlsdr), so it gets a neutral label instead of "hackrf fw" — including
    // in observer mode. All labels are exactly 10 columns so top_band_gap stays
    // valid.
    let (fw_val, fw_label): (std::sync::Arc<str>, &str) = if state.caps.gain.is_single() {
        let v = if state.observer.active { "—" } else { "librtlsdr" };
        (std::sync::Arc::from(v), "rtl-sdr   ")
    } else if state.observer.active {
        (std::sync::Arc::from("—"), "hackrf fw ")
    } else {
        let is_mayhem = state.system.fw_version.starts_with("n_")
            || (state.system.fw_version.starts_with('v')
                && state.system.fw_version.chars().nth(1).map_or(false, |c| c.is_ascii_digit()));
        let label = if is_mayhem { "mayhem fw " } else { "hackrf fw " };
        (state.system.fw_version.clone(), label)
    };
    let fw_color = if state.observer.active { theme.label } else { theme.value };
    let fw_len = fw_val.chars().count();

    // --- AMP value (always 3 terminal columns) ---
    let (amp_val, amp_color) = if state.observer.active {
        ("—  ".to_string(), theme.label)
    } else if state.radio.amp_enabled {
        ("ON ".to_string(), theme.value_hi)
    } else {
        ("OFF".to_string(), theme.label)
    };

    // --- USB value (always 9 terminal columns) ---
    let (usb_val, usb_color) = if state.radio.hw_streaming && state.radio.current_throughput_bps > 0 {
        let mb = state.radio.current_throughput_bps as f64 / 1_000_000.0;
        (format!("{:4.1} MB/s", mb), theme.value)
    } else {
        ("—        ".to_string(), theme.label)  // 1 + 8 spaces = 9 chars
    };

    // --- Gap ---
    let board_len = state.system.board_name.chars().count();
    let gap = top_band_gap(board_len, badge_len, fw_len,
                           amp_val.chars().count(), usb_val.chars().count(), inner_width);

    Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!(" {} ", state.system.board_name),
            Style::default()
                .fg(theme.value_hi)
                .bg(Color::Rgb(20, 25, 38))
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            badge_text,
            Style::default().fg(badge_fg).bg(badge_bg).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(fw_label, Style::default().fg(theme.label)),
        Span::styled(fw_val.to_string(), Style::default().fg(fw_color)),
        leader(gap, theme.border_dim),
        // HackRF's RF amp or RTL-SDR's tuner AGC — both 3-char labels, so the
        // "{label} " field stays 4 columns and top_band_gap remains valid.
        Span::styled(format!("{} ", state.caps.gain.boost_label()), Style::default().fg(theme.label)),
        Span::styled(amp_val, Style::default().fg(amp_color)),
        Span::raw("  ·  "),
        Span::styled("USB ", Style::default().fg(theme.label)),
        Span::styled(usb_val, Style::default().fg(usb_color)),
        Span::raw("  "),
    ])
}

/// Builds the ├──── FREQUENCY ────┤ line.
/// `outer_width` is the FULL panel width (including border chars), not the inner width.
/// The returned Line must be rendered at the outer Rect so ├/┤ overwrite the │ border chars.
fn separator_line(theme: &crate::Theme, outer_width: u16) -> Line<'static> {
    let label = " FREQUENCY ";
    let label_len = label.chars().count();
    let fill = (outer_width as usize).saturating_sub(2 + label_len);  // 2 for ├ and ┤
    let left_fill = fill / 2;
    let right_fill = fill - left_fill;
    Line::from(vec![
        Span::styled("├", Style::default().fg(theme.border_dim)),
        Span::styled("─".repeat(left_fill), Style::default().fg(theme.border_default)),
        Span::styled(label, Style::default().fg(theme.border_dim)),
        Span::styled("─".repeat(right_fill), Style::default().fg(theme.border_default)),
        Span::styled("┤", Style::default().fg(theme.border_dim)),
    ])
}

/// Frequency · sample-rate on the left, gain bars right-aligned. Left block
/// (freq + SR): 31 chars. Right block: 42 chars — either HackRF's LNA + VGA, or a
/// single-tuner stage (RTL-SDR) with the second-stage region blanked to the same
/// width so the gap math and right-alignment hold for both.
fn bottom_band_line(state: &SdrMetrics, theme: &crate::Theme, inner_width: u16) -> Line<'static> {
    let active = state.radio.hw_streaming && !state.observer.active;
    let gm = &state.caps.gain;

    // Sample rate: right-padded to 4 chars
    let sr_str = format!("{:4.1}", state.radio.config_sample_rate / 1_000_000.0);

    let freq_color = if state.observer.active { theme.label } else { theme.border_accent };
    let val_color  = if active { theme.value } else { theme.label };
    let lna_color  = if active { theme.status_ok } else { theme.label };
    let vga_color  = if active { theme.status_warn } else { theme.label };
    let dim        = theme.border_dim;

    // Left block: segmented VFO readout + unit + sample-rate. Its width varies
    // with the number of MHz digits and the active-digit underline, so it is
    // measured (below) rather than assumed, and the trailing gap fills the rest.
    let mut left_spans = vec![Span::raw("  ")];
    left_spans.extend(vfo_spans(state.radio.frequency, state.spectrum.step_hz,
                                freq_color, theme.label, theme.value_hi));
    left_spans.extend([
        Span::raw(" "),
        Span::styled("MHz", Style::default().fg(theme.label)),
        Span::raw("    "),
        Span::styled("SR ", Style::default().fg(theme.label)),
        Span::styled(sr_str, Style::default().fg(val_color)),
        Span::styled(" Msps", Style::default().fg(theme.label)),
    ]);
    let left_w: usize = left_spans.iter().map(|s| s.width()).sum();

    // right: primary "LNA/TUN "(4) + bar(8) + " "(1) + val(2) + " dB"(3) + "    "(4)  = 22
    //      + second stage "VGA "(4) + bar(8) + " "(1) + val(2) + " dB"(3) + "  "(2)   = 20  (blank on RTL)
    let right = 22 + 20;
    let gap = (inner_width as usize).saturating_sub(left_w + right);

    // Primary stage: HackRF LNA / RTL-SDR tuner.
    let (p_filled, p_empty) = gain_bar(state.radio.lna_gain, gm.primary_max_db(), 8);
    let p_str = format!("{:2}", state.radio.lna_gain);
    let p_label = if gm.is_single() { "TUN " } else { "LNA " };

    let mut spans = left_spans;
    spans.extend([
        leader(gap, theme.border_dim),
        Span::styled(p_label, Style::default().fg(theme.label)),
        Span::styled(p_filled, Style::default().fg(lna_color)),
        Span::styled(p_empty, Style::default().fg(dim)),
        Span::raw(" "),
        Span::styled(p_str, Style::default().fg(val_color)),
        Span::styled(" dB", Style::default().fg(theme.label)),
        Span::raw("    "),
    ]);

    if gm.has_second_stage() {
        let (vga_filled, vga_empty) = gain_bar(state.radio.vga_gain, 62, 8);
        let vga_str = format!("{:2}", state.radio.vga_gain);
        spans.extend([
            Span::styled("VGA ", Style::default().fg(theme.label)),
            Span::styled(vga_filled, Style::default().fg(vga_color)),
            Span::styled(vga_empty, Style::default().fg(dim)),
            Span::raw(" "),
            Span::styled(vga_str, Style::default().fg(val_color)),
            Span::styled(" dB", Style::default().fg(theme.label)),
            Span::raw("  "),
        ]);
    } else {
        // Single-tuner device: blank the 20-col second-stage region to keep width.
        spans.push(Span::raw(" ".repeat(20)));
    }

    Line::from(spans)
}

impl Panel for HeaderPanel {
    fn name(&self) -> &'static str { "header" }
    fn min_size(&self) -> (u16, u16) { (60, 5) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        let block = chrome::deck_block(theme.border_dim)
            .title(chrome::title("Radio", theme.label, theme.border_dim));
        let inner = block.inner(area);
        f.render_widget(block, area);

        // inner.height == 3 when area.height == 5
        // Row positions (absolute y):
        //   inner.y     → top band
        //   inner.y + 1 → separator (rendered at outer width to overwrite the │ border chars)
        //   inner.y + 2 → bottom band

        if inner.height < 3 { return; }
        let top_area = Rect { x: inner.x, y: inner.y,     width: inner.width, height: 1 };
        let sep_area = Rect { x: area.x,  y: inner.y + 1, width: area.width,  height: 1 };
        let bot_area = Rect { x: inner.x, y: inner.y + 2, width: inner.width, height: 1 };

        f.render_widget(Paragraph::new(top_band_line(state, theme, inner.width)), top_area);
        f.render_widget(Paragraph::new(separator_line(theme, area.width)), sep_area);
        f.render_widget(Paragraph::new(bottom_band_line(state, theme, inner.width)), bot_area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn gain_bar_zero_gain_all_empty() {
        let (filled, empty) = gain_bar(0, 40, 8);
        assert_eq!(filled, "");
        assert_eq!(empty, "▯▯▯▯▯▯▯▯");
    }

    #[test]
    fn gain_bar_full_gain_all_filled() {
        let (filled, empty) = gain_bar(40, 40, 8);
        assert_eq!(filled, "▮▮▮▮▮▮▮▮");
        assert_eq!(empty, "");
    }

    #[test]
    fn step_place_exp_maps_steps_to_digit_place() {
        // decade steps land exactly on their digit
        assert_eq!(step_place_exp(1_000),     3); // 1 kHz
        assert_eq!(step_place_exp(10_000),    4); // 10 kHz
        assert_eq!(step_place_exp(100_000),   5); // 100 kHz
        assert_eq!(step_place_exp(1_000_000), 6); // 1 MHz
        assert_eq!(step_place_exp(10_000_000),7); // 10 MHz
        // non-decade steps collapse onto their leading digit's place
        assert_eq!(step_place_exp(5_000),   3);
        assert_eq!(step_place_exp(25_000),  4);
        assert_eq!(step_place_exp(500_000), 5);
        assert_eq!(step_place_exp(5_000_000), 6);
    }

    #[test]
    fn vfo_underlines_the_active_digit() {
        let t = Theme::sdr();
        // 145.500 MHz, 10 kHz step → the 10-kHz digit is the first decimal-2 ('0'
        // in ".50"). Exactly one span carries UNDERLINED.
        let spans = vfo_spans(145_500_000, 10_000, t.border_accent, t.label, t.value_hi);
        let underlined: Vec<&str> = spans.iter()
            .filter(|s| s.style.add_modifier.contains(Modifier::UNDERLINED))
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(underlined.len(), 1, "exactly one active digit");
        // "145.500": frac index 1 (5→exp5,4→exp4) → the '0' after the '5'
        assert_eq!(underlined[0], "0");
        // active digit is brightened, not the plain accent
        let act = spans.iter().find(|s| s.style.add_modifier.contains(Modifier::UNDERLINED)).unwrap();
        assert_eq!(act.style.fg, Some(t.value_hi));
    }

    #[test]
    fn vfo_step_above_screen_underlines_nothing() {
        let t = Theme::sdr();
        // 5 MHz, 10 MHz step → tens-of-MHz digit, which doesn't exist → no underline
        let spans = vfo_spans(5_000_000, 10_000_000, t.border_accent, t.label, t.value_hi);
        let any = spans.iter().any(|s| s.style.add_modifier.contains(Modifier::UNDERLINED));
        assert!(!any, "active digit off-screen → nothing underlined");
    }

    #[test]
    fn gain_bar_half_gain() {
        let (filled, empty) = gain_bar(20, 40, 8);
        assert_eq!(filled.chars().count(), 4);
        assert_eq!(empty.chars().count(), 4);
    }

    #[test]
    fn gain_bar_total_always_equals_width() {
        for gain in [0u32, 1, 16, 20, 40] {
            let (f, e) = gain_bar(gain, 40, 8);
            assert_eq!(f.chars().count() + e.chars().count(), 8,
                "gain={gain}: filled({}) + empty({}) != 8", f.chars().count(), e.chars().count());
        }
    }

    #[test]
    fn top_band_gap_rx_state() {
        // HackRF One (len=10), badge " ● RX " (len=6), fw "2024.02.1" (len=9), inner=78
        // amp_val "ON " (3), usb_val "10.0 MB/s" (9)
        assert_eq!(top_band_gap(10, 6, 9, 3, 9, 78), 9);
    }

    #[test]
    fn top_band_gap_idle_state() {
        // badge " ○ IDLE " is 2 chars wider than RX → gap shrinks by 2
        assert_eq!(top_band_gap(10, 8, 9, 3, 9, 78), 7);
    }

    #[test]
    fn top_band_gap_observer_state() {
        // badge " ◈ OBSERVER " (len=12), fw "—" (len=1)
        assert_eq!(top_band_gap(10, 12, 1, 3, 9, 78), 11);
    }
}
