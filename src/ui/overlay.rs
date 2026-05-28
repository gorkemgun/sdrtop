use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_help(f: &mut Frame) {
    let area = centered_rect(62, 32, f.size());

    let text = "\
 [Q]        Quit\n\
 [SPACE]    Start / Stop RX\n\
 [↑] [↓]    LNA gain  +8 / −8 dB  (0–40 dB)\n\
 [[] []]    VGA gain  −2 / +2 dB  (0–62 dB)\n\
 [A]        Toggle AMP\n\
 [F]        Enter frequency (MHz)\n\
 [S]        Enter sample rate (2–20 MHz)\n\
 [R]        Reset all to defaults\n\
 [P]        Cycle presets\n\
 [1]        Preset: minimal\n\
 [2]        Preset: monitoring\n\
 [3]        Preset: spectrum\n\
 [4]        Preset: waterfall\n\
 [5]        Preset: spectrum+waterfall\n\
 [6]        Preset: lab\n\
 [W]        Pause / resume waterfall\n\
 [?]        Toggle this help\n\
\n\
 Panel focus:\n\
   [E] Spectrum  [O] Waterfall  [H] Hardware Health\n\
   [C] RF Chain  [M] Signal     [I] IQ Diag  [G] Gains\n\
   Esc  Exit focus mode\n\
\n\
 --theme <name>:  sdr | nord | dracula | gruvbox | catppuccin | solarized\n\
\n\
 In frequency / sample rate input mode:\n\
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
