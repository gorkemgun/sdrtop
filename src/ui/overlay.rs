use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_help(f: &mut Frame) {
    let area = centered_rect(52, 18, f.size());

    let text = "\
 [Q]        Quit\n\
 [SPACE]    Start / Stop RX\n\
 [↑] [↓]    LNA gain  +8 / −8 dB  (0–40 dB)\n\
 [[] []]    VGA gain  −2 / +2 dB  (0–62 dB)\n\
 [A]        Toggle AMP\n\
 [F]        Enter frequency (MHz)\n\
 [R]        Reset all to defaults\n\
 [P]        Cycle presets\n\
 [1]        Preset: minimal\n\
 [2]        Preset: monitoring\n\
 [3]        Preset: spectrum\n\
 [?]        Toggle this help\n\
\n\
 In frequency input mode:\n\
   digits / .    type value\n\
   Backspace     delete last char\n\
   Enter         confirm\n\
   Esc           cancel\
";

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title(" Help — press [?] to close ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Left),
        area,
    );
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}
