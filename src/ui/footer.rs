use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::{InputMode, SdrMetrics};

pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics) {
    let text = match m.input_mode {
        InputMode::Normal => {
            " [Q] Quit | [SPACE] RX | [↑↓] LNA | [[] VGA | [A] AMP | [F] Freq | [R] Reset | [?] Help ".to_string()
        }
        InputMode::FrequencyInput => {
            format!(" Frequency (MHz): [{}▌] | [Enter] confirm | [Esc] cancel ", m.input_buf)
        }
    };
    let footer = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(footer, area);
}

use super::panel::Panel;

pub struct FooterPanel;

impl Panel for FooterPanel {
    fn name(&self) -> &'static str { "footer" }
    fn min_size(&self) -> (u16, u16) { (40, 3) }
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        render(f, area, state);
    }
}
