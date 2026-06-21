//! `IqConstellationPanel` — 2-D braille dot-cloud of recent I/Q samples.
//!
//! Each frame shows up to [`CONSTELLATION_CAP`] normalised (I, Q) pairs from
//! the RX hot-path, decimated 1 : 1024. The cloud's position reveals the DC
//! offset; its shape reveals amplitude/phase imbalance (circular = perfect,
//! elliptical = amplitude imbalance, tilted = phase imbalance). A unit circle
//! and faint I/Q axes give a fixed reference frame.

use std::f64::consts::PI;

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Points},
        Block, BorderType, Borders, Paragraph,
    },
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct IqConstellationPanel;

/// Canvas coordinate half-extent — slightly wider than the unit circle so the
/// circle border and labels are not clipped.
const BOUND: f64 = 1.3;

/// Number of line segments used to approximate the unit circle.
const CIRCLE_SEGS: usize = 48;

impl Panel for IqConstellationPanel {
    fn name(&self) -> &'static str { "iq_constellation" }
    fn min_size(&self) -> (u16, u16) { (18, 10) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = !state.radio.hw_streaming;
        let border_color = if focused { theme.border_focused }
            else if stale { theme.stale }
            else { theme.border_default };

        let title_line = Line::from(Span::styled(
            " IQ Constellation ",
            Style::default().fg(theme.label).add_modifier(Modifier::BOLD),
        ));
        let block = Block::default()
            .title(title_line)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if stale {
            f.render_widget(
                Paragraph::new(Span::styled("Waiting for RX\u{2026}", Style::default().fg(theme.label))),
                inner,
            );
            return;
        }

        if state.iq.constellation.is_empty() {
            f.render_widget(
                Paragraph::new(Span::styled("No samples yet\u{2026}", Style::default().fg(theme.label))),
                inner,
            );
            return;
        }

        // Pre-collect coords into an owned Vec so the closure can borrow them.
        let coords: Vec<(f64, f64)> = state.iq.constellation.iter()
            .map(|&(i, q)| (i as f64, q as f64))
            .collect();

        let dc_i = state.iq.dc_offset_i as f64;
        let dc_q = state.iq.dc_offset_q as f64;

        let axis_color  = theme.border_dim;
        let circle_color = theme.border_dim;
        let point_color = theme.value_hi;
        let dc_color    = theme.status_warn;

        f.render_widget(
            Canvas::default()
                .x_bounds([-BOUND, BOUND])
                .y_bounds([-BOUND, BOUND])
                .paint(move |ctx| {
                    // I-axis (horizontal)
                    ctx.draw(&CanvasLine { x1: -BOUND, y1: 0.0, x2: BOUND, y2: 0.0, color: axis_color });
                    // Q-axis (vertical)
                    ctx.draw(&CanvasLine { x1: 0.0, y1: -BOUND, x2: 0.0, y2: BOUND, color: axis_color });

                    // Unit circle — CIRCLE_SEGS line segments
                    for k in 0..CIRCLE_SEGS {
                        let a0 = 2.0 * PI * k as f64 / CIRCLE_SEGS as f64;
                        let a1 = 2.0 * PI * (k + 1) as f64 / CIRCLE_SEGS as f64;
                        ctx.draw(&CanvasLine {
                            x1: a0.cos(), y1: a0.sin(),
                            x2: a1.cos(), y2: a1.sin(),
                            color: circle_color,
                        });
                    }

                    // Constellation cloud
                    ctx.draw(&Points { coords: &coords, color: point_color });

                    // DC offset crosshair (short arms centred on the measured offset)
                    let arm = 0.07;
                    ctx.draw(&CanvasLine { x1: dc_i - arm, y1: dc_q,       x2: dc_i + arm, y2: dc_q,       color: dc_color });
                    ctx.draw(&CanvasLine { x1: dc_i,       y1: dc_q - arm, x2: dc_i,       y2: dc_q + arm, color: dc_color });
                }),
            inner,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_name_and_min_size() {
        let p = IqConstellationPanel;
        assert_eq!(p.name(), "iq_constellation");
        let (w, h) = p.min_size();
        assert!(w > 0 && h > 0);
    }

    #[test]
    fn circle_segs_constant_is_positive_even() {
        assert!(CIRCLE_SEGS > 0);
        assert_eq!(CIRCLE_SEGS % 2, 0, "even number of segments gives symmetric circle");
    }
}
