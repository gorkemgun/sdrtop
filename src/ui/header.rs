use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, area: Rect, board_name: &str, fw: &str, serial: &str) {
    let header = Paragraph::new(format!(" {} | FW: {} | S/N: {} ", board_name, fw, serial))
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(header, area);
}

use super::panel::Panel;
use crate::state::SdrMetrics;

pub struct HeaderPanel {
    pub board_name: String,
    pub fw_version: String,
    pub serial: String,
}

impl Panel for HeaderPanel {
    fn name(&self) -> &'static str { "header" }
    fn min_size(&self) -> (u16, u16) { (40, 3) }
    fn render(&self, f: &mut Frame, area: Rect, _state: &SdrMetrics) {
        render(f, area, &self.board_name, &self.fw_version, &self.serial);
    }
}
