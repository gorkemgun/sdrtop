//! Shared building blocks for the micro ecosystem panels (`micro_panel`,
//! `micro_signal`, `micro_gain`, `micro_health`). Keeps the status colors,
//! formatters, the status badge, and the character bar in one place so the four
//! panels stay visually consistent.

use ratatui::{
    style::{Color, Style},
    text::Span,
};

use crate::state::SdrMetrics;

// ── Status colors ───────────────────────────────────────────────────────────

pub fn snr_color(db: f32, theme: &crate::Theme) -> Color {
    if db >= 20.0 { theme.status_ok } else if db >= 10.0 { theme.status_warn } else { theme.status_crit }
}
pub fn sat_color(pct: f32, theme: &crate::Theme) -> Color {
    if pct < 1.0 { theme.status_ok } else if pct < 5.0 { theme.status_warn } else { theme.status_crit }
}
pub fn drop_color(drops: u64, theme: &crate::Theme) -> Color {
    if drops == 0 { theme.status_ok } else if drops < 10 { theme.status_warn } else { theme.status_crit }
}
pub fn buf_color(pct: f32, theme: &crate::Theme) -> Color {
    if pct < 50.0 { theme.status_ok } else if pct < 80.0 { theme.status_warn } else { theme.status_crit }
}

// ── Formatters ──────────────────────────────────────────────────────────────

pub fn fmt_rbw(hz: f64) -> String {
    if hz >= 1_000.0 { format!("{:.1} kHz", hz / 1_000.0) } else { format!("{:.0} Hz", hz) }
}

pub fn fmt_freq_mhz(hz: u64) -> String {
    format!("{:.3} MHz", hz as f64 / 1_000_000.0)
}

/// `152 kHz` / `1.2 MHz` style bandwidth.
pub fn fmt_bw(hz: u64) -> String {
    if hz >= 1_000_000 { format!("{:.2} MHz", hz as f64 / 1_000_000.0) }
    else if hz >= 1_000 { format!("{} kHz", hz / 1_000) }
    else { format!("{} Hz", hz) }
}

// ── Shared spans ────────────────────────────────────────────────────────────

/// `● RX` (green) when streaming, `○ IDLE` (yellow) otherwise. Two spans: the
/// dot and the word, both carrying the status color so they read on monochrome.
pub fn status_badge(state: &SdrMetrics, theme: &crate::Theme) -> [Span<'static>; 2] {
    let (dot, col, word) = if state.radio.rx_enabled {
        ("●", theme.status_ok, " RX")
    } else {
        ("○", theme.status_warn, " IDLE")
    };
    [
        Span::styled(dot, Style::default().fg(col)),
        Span::styled(word, Style::default().fg(col)),
    ]
}

/// Whether FFT-derived signal data (SNR, PWR, NF, RBW) is stale — no frame, or
/// the last one is older than 500 ms.
pub fn fft_stale(state: &SdrMetrics) -> bool {
    state.waterfall.last_fft.as_ref()
        .map(|fr| fr.timestamp.elapsed().as_millis() > 500)
        .unwrap_or(true)
}

/// A character bar `████░░░░` of `width` cells: filled (in `color`) for `ratio`,
/// empty (dim) for the rest. Two spans.
pub fn bar_spans(ratio: f64, width: usize, color: Color, theme: &crate::Theme) -> [Span<'static>; 2] {
    let ratio  = ratio.clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    let empty  = width.saturating_sub(filled);
    [
        Span::styled("█".repeat(filled), Style::default().fg(color)),
        Span::styled("░".repeat(empty),  Style::default().fg(theme.border_dim)),
    ]
}

/// Block-character ticks for inline sparklines, low → high.
const SPARK_TICKS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Inline sparkline string from the most recent `width` samples, auto-scaled to
/// the window maximum. An all-zero (or empty-window) input renders as the
/// lowest tick so the row still reads as a flat baseline.
pub fn sparkline(samples: &[f64], width: usize) -> String {
    if samples.is_empty() || width == 0 { return String::new(); }
    let start = samples.len().saturating_sub(width);
    let slice = &samples[start..];
    let max = slice.iter().cloned().fold(0.0f64, f64::max).max(1e-9);
    slice.iter()
        .map(|&v| {
            let idx = ((v / max) * (SPARK_TICKS.len() - 1) as f64).round().clamp(0.0, 7.0) as usize;
            SPARK_TICKS[idx]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn snr_color_thresholds() {
        let t = Theme::sdr();
        assert_eq!(snr_color(25.0, &t), t.status_ok);
        assert_eq!(snr_color(15.0, &t), t.status_warn);
        assert_eq!(snr_color(5.0,  &t), t.status_crit);
    }

    #[test]
    fn sat_and_drop_and_buf_thresholds() {
        let t = Theme::sdr();
        assert_eq!(sat_color(0.5, &t), t.status_ok);
        assert_eq!(sat_color(8.0, &t), t.status_crit);
        assert_eq!(drop_color(0, &t), t.status_ok);
        assert_eq!(drop_color(15, &t), t.status_crit);
        assert_eq!(buf_color(10.0, &t), t.status_ok);
        assert_eq!(buf_color(90.0, &t), t.status_crit);
    }

    #[test]
    fn fmt_helpers() {
        assert_eq!(fmt_rbw(9_800.0), "9.8 kHz");
        assert_eq!(fmt_rbw(800.0), "800 Hz");
        assert_eq!(fmt_freq_mhz(433_920_000), "433.920 MHz");
        assert_eq!(fmt_bw(152_000), "152 kHz");
        assert_eq!(fmt_bw(1_200_000), "1.20 MHz");
    }

    #[test]
    fn sparkline_scales_and_handles_edges() {
        assert_eq!(sparkline(&[], 8), "");
        // All-zero → flat baseline of lowest ticks.
        assert_eq!(sparkline(&[0.0, 0.0, 0.0], 8), "▁▁▁");
        // Max sample maps to the top tick; only the last `width` are used.
        let s = sparkline(&[0.0, 10.0], 2);
        assert_eq!(s.chars().count(), 2);
        assert_eq!(s.chars().nth(1), Some('█'));
        // Window keeps only the most recent `width` samples.
        assert_eq!(sparkline(&[5.0, 5.0, 5.0, 5.0], 2).chars().count(), 2);
    }

    #[test]
    fn bar_fills_proportionally() {
        let t = Theme::sdr();
        let [filled, empty] = bar_spans(0.5, 10, t.status_ok, &t);
        assert_eq!(filled.content.chars().count(), 5);
        assert_eq!(empty.content.chars().count(), 5);
        // Clamps above 1.0.
        let [filled, _] = bar_spans(2.0, 8, t.status_ok, &t);
        assert_eq!(filled.content.chars().count(), 8);
    }
}
