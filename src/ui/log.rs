use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::state::SdrMetrics;

pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics) {
    let log_lines: Vec<&str> = m.log.iter().map(|s| s.as_str()).collect();
    let log_text = log_lines.join("\n");
    let panel = Paragraph::new(log_text)
        .block(Block::default().title(" Log ").borders(Borders::ALL))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(panel, area);
}

use super::panel::Panel;

pub struct LogPanel;

impl Panel for LogPanel {
    fn name(&self) -> &'static str { "log" }
    fn min_size(&self) -> (u16, u16) { (20, 7) }
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        render(f, area, state);
    }
}
