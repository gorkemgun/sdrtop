use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::{InputMode, SdrMetrics};

pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics) {
    let text = if m.observer_mode {
        " Observer Mode — Hardware controls disabled.  [Q] Quit  [?] Help ".to_string()
    } else {
        match m.input_mode {
            InputMode::Normal => {
                " [Q] Quit | [SPACE] RX | [↑↓] LNA | [[] VGA | [A] AMP | [F] Freq | [S] Rate | [R] Reset | [?] Help ".to_string()
            }
            InputMode::FrequencyInput => {
                format!(" Frequency (MHz): [{}▌] | [Enter] confirm | [Esc] cancel ", m.input_buf)
            }
            InputMode::SampleRateInput => {
                format!(" Sample rate (2–20 MHz): [{}▌] | [Enter] confirm | [Esc] cancel ", m.input_buf)
            }
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
