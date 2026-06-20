use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine},
        Paragraph,
    },
    Frame,
};

use crate::state::THROUGHPUT_HISTORY_LEN;

/// Full-block horizontal bar — same visual language as the header's LNA/VGA gain bars.
/// Renders into a single terminal row: `label ████░░░░ value_str`
pub fn draw_hbar(
    f: &mut Frame,
    area: Rect,
    ratio: f64,
    label: &str,
    value_str: &str,
    color: Color,
    theme: &crate::Theme,
) {
    let ratio   = ratio.clamp(0.0, 1.0);
    let label_w = label.chars().count() as u16;
    let val_w   = (value_str.chars().count() + 1) as u16; // +1 space separator
    let bar_w   = area.width.saturating_sub(label_w + val_w) as usize;
    let filled  = (ratio * bar_w as f64).round() as usize;
    let empty   = bar_w.saturating_sub(filled);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(label.to_string(),     Style::default().fg(theme.label)),
            Span::styled("█".repeat(filled),    Style::default().fg(color)),
            Span::styled("░".repeat(empty),     Style::default().fg(theme.border_dim)),
            Span::raw(" "),
            Span::styled(value_str.to_string(), Style::default().fg(color)),
        ])),
        area,
    );
}

/// A one-line inline sparkline from the most recent `width` samples, drawn with
/// the `▁▂▃▄▅▆▇█` ramp and auto-scaled to the window's own min..max. Returns a
/// string of exactly `width` columns (left-padded with spaces when there are
/// fewer samples than `width`). Non-finite samples render as a gap. Used by the
/// command rail's metric rows.
pub fn sparkline(data: &[f32], width: usize) -> String {
    const RAMP: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if width == 0 { return String::new(); }
    if data.is_empty() { return " ".repeat(width); }

    let start  = data.len().saturating_sub(width);
    let window = &data[start..];
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    for &v in window {
        if v.is_finite() { lo = lo.min(v); hi = hi.max(v); }
    }
    if !lo.is_finite() || !hi.is_finite() { return " ".repeat(width); }
    let range = (hi - lo).max(1e-6);

    let mut s = String::with_capacity(width);
    for _ in 0..width.saturating_sub(window.len()) { s.push(' '); }
    for &v in window {
        if !v.is_finite() { s.push(' '); continue; }
        let t = (((v - lo) / range) * 7.0).round().clamp(0.0, 7.0) as usize;
        s.push(RAMP[t]);
    }
    s
}

/// Canvas filled-column graph — same style as the spectrum panel (filled columns + outline).
/// Accepts a plain `&[u64]` slice. Scales automatically to the data maximum.
pub fn draw_mini_graph(f: &mut Frame, area: Rect, data: &[u64], color: Color) {
    if area.height == 0 || area.width < 2 || data.is_empty() { return; }

    let values: Vec<f64> = data.iter().map(|&v| v as f64).collect();
    let n       = values.len();
    let max_val = values.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
    let max_n   = THROUGHPUT_HISTORY_LEN as f64;
    let x_off   = max_n - n as f64;

    f.render_widget(
        Canvas::default()
            .x_bounds([0.0, max_n])
            .y_bounds([0.0, max_val])
            .paint(move |ctx| {
                // Filled columns
                for (i, &val) in values.iter().enumerate() {
                    let x = x_off + i as f64;
                    ctx.draw(&CanvasLine { x1: x, y1: 0.0, x2: x, y2: val, color });
                }
                // Outline connecting column tops
                for i in 1..n {
                    ctx.draw(&CanvasLine {
                        x1: x_off + (i - 1) as f64, y1: values[i - 1],
                        x2: x_off +  i      as f64, y2: values[i],
                        color,
                    });
                }
            }),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every sparkline char is single-column, so chars().count() == display width.
    #[test]
    fn sparkline_is_always_width_columns() {
        assert_eq!(sparkline(&[], 6).chars().count(), 6);
        assert_eq!(sparkline(&[1.0], 6).chars().count(), 6);
        assert_eq!(sparkline(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0], 6).chars().count(), 6);
        assert_eq!(sparkline(&[1.0; 3], 0).chars().count(), 0);
    }

    #[test]
    fn sparkline_maps_extremes_to_ramp_ends() {
        // A rising ramp ends low→high: first non-space is ▁, last is █.
        let s: Vec<char> = sparkline(&[0.0, 1.0, 2.0, 3.0], 4).chars().collect();
        assert_eq!(s[0], '▁');
        assert_eq!(s[3], '█');
    }

    #[test]
    fn sparkline_flat_series_does_not_panic_or_spike() {
        // Equal samples → range floored to 1e-6, all map to the lowest bar.
        let s = sparkline(&[-76.3; 6], 6);
        assert!(s.chars().all(|c| c == '▁'));
    }

    #[test]
    fn sparkline_left_pads_when_too_few_samples() {
        let s = sparkline(&[5.0, 9.0], 6);
        assert!(s.starts_with("    "), "got {s:?}");
    }
}
