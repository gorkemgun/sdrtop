use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct IqDiagnosticsPanel;

fn offset_color(abs_val: f32) -> Color {
    if abs_val > 0.02       { Color::Red    }
    else if abs_val > 0.005 { Color::Yellow }
    else                    { Color::Green  }
}

fn imbalance_color(abs_db: f32) -> Color {
    if abs_db > 3.0      { Color::Red    }
    else if abs_db > 1.0 { Color::Yellow }
    else                 { Color::Green  }
}

impl Panel for IqDiagnosticsPanel {
    fn name(&self) -> &'static str { "iq_diagnostics" }
    fn min_size(&self) -> (u16, u16) { (30, 6) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" IQ Diagnostics ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let max_offset = state.dc_offset_i.abs().max(state.dc_offset_q.abs());
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "DC offset  I: {:+.4}  Q: {:+.4}",
                    state.dc_offset_i, state.dc_offset_q
                ),
                Style::default().fg(offset_color(max_offset)),
            )),
            rows[0],
        );

        let abs_imbalance = state.iq_imbalance_db.abs();
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("IQ imbalance: {:+.2} dB", state.iq_imbalance_db),
                Style::default().fg(imbalance_color(abs_imbalance)),
            )),
            rows[1],
        );

        let hint = if abs_imbalance < 1.0        { "OK — channels balanced" }
            else if state.iq_imbalance_db > 0.0  { "I channel stronger" }
            else                                  { "Q channel stronger" };
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("  \u{2192} {}", hint),
                Style::default().fg(Color::DarkGray),
            )),
            rows[2],
        );
    }
}
