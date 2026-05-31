use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

struct DeviceSelector {
    devices: Vec<String>,
    state: ListState,
}

impl DeviceSelector {
    fn new(devices: Vec<String>) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self { devices, state }
    }

    fn draw(&mut self, f: &mut Frame) {
        let area = f.size();

        f.render_widget(
            Block::default().style(Style::default().bg(Color::Rgb(15, 15, 25))),
            area,
        );

        let count = self.devices.len() as u16;
        let dialog_h = (count + 4).min(area.height.saturating_sub(4));
        let dialog_w = 64u16.min(area.width.saturating_sub(4));

        let dialog_area = {
            let vert = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(area.height.saturating_sub(dialog_h) / 2),
                    Constraint::Length(dialog_h),
                    Constraint::Min(0),
                ])
                .split(area);
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(area.width.saturating_sub(dialog_w) / 2),
                    Constraint::Length(dialog_w),
                    Constraint::Min(0),
                ])
                .split(vert[1])[1]
        };

        f.render_widget(Clear, dialog_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(dialog_area);

        let title = if self.devices.len() == 1 {
            " 1 device found — press Enter ".to_string()
        } else {
            format!(" {} devices found — select one ", self.devices.len())
        };

        let items: Vec<ListItem> = self
            .devices
            .iter()
            .enumerate()
            .map(|(i, serial)| ListItem::new(format!("  [{i}]  {serial}")))
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .title_alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Cyan).bg(Color::Rgb(20, 20, 35))),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(15, 15, 25))
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("► ");

        f.render_stateful_widget(list, chunks[0], &mut self.state);

        let hint = Paragraph::new("  ↑/↓ navigate    Enter select    q quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        f.render_widget(hint, chunks[1]);
    }
}

/// Runs a TUI device picker. Returns the selected index, or `None` if the user quit.
pub fn run<B: Backend>(
    devices: Vec<String>,
    terminal: &mut Terminal<B>,
) -> anyhow::Result<Option<usize>> {
    let mut sel = DeviceSelector::new(devices);
    loop {
        terminal.draw(|f| sel.draw(f))?;
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    let i = sel.state.selected().unwrap_or(0);
                    sel.state.select(Some(i.saturating_sub(1)));
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    let i = sel.state.selected().unwrap_or(0);
                    sel.state
                        .select(Some((i + 1).min(sel.devices.len().saturating_sub(1))));
                }
                KeyCode::Enter => return Ok(sel.state.selected()),
                KeyCode::Char('q') | KeyCode::Esc => return Ok(None),
                _ => {}
            }
        }
    }
}
