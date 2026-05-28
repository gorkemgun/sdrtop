use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use super::panel::Panel;

pub struct HeaderPanel;

impl Panel for HeaderPanel {
    fn name(&self) -> &'static str { "header" }
    fn min_size(&self) -> (u16, u16) { (40, 3) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        let (status, status_color) = if state.observer_mode {
            ("Observer", theme.observer)
        } else if state.hw_streaming {
            ("RX", theme.status_ok)
        } else {
            ("IDLE", theme.status_warn)
        };

        let freq = format!("{:.3} MHz", state.frequency as f64 / 1_000_000.0);
        let sep = Span::styled(" │ ", Style::default().fg(theme.border_dim));

        let line = Line::from(vec![
            Span::styled(
                format!(" {} ", state.board_name),
                Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD),
            ),
            sep.clone(),
            Span::styled(format!(" {} ", state.fw_version), Style::default().fg(theme.label)),
            sep.clone(),
            Span::styled(format!(" {} ", freq), Style::default().fg(theme.value)),
            sep.clone(),
            Span::styled(
                format!(" {} ", status),
                Style::default().fg(status_color).add_modifier(Modifier::BOLD),
            ),
        ]);

        f.render_widget(
            Paragraph::new(line)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(theme.border_dim)),
                )
                .alignment(Alignment::Center),
            area,
        );
    }
}
