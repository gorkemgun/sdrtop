use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Enumerate all connected HackRF devices
    // This allows us to see how many boards are available before picking one
    println!("Searching for HackRF devices...");
    
    // In a real scenario with the hackrf crate, we might use a device list.
    // For now, we attempt to open the default device and handle the result.
    // If multiple devices are needed, we would use hackrf::open_with_serial().
    let board = hackrf::open().map_err(|e| {
        anyhow::anyhow!("Failed to open HackRF: {}. Check USB connection and udev rules.", e)
    })?;

    // In the future, this is where we would implement a selection list
    // if more than one serial number was detected.
    println!("Found HackRF device!");

    // Retrieve basic hardware information
    let version = board.version()?;
    let board_id = board.board_id_read()?;

    // Setup terminal for TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main application loop
    let app_result = run_app(&mut terminal, &version, board_id);

    // Restore terminal state
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = app_result {
        println!("Application error: {:?}", err);
    }

    Ok(())
}

/// Main TUI render loop and event handling
fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    fw_version: &str,
    board_id: hackrf::BoardId,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let size = f.size();

            // Define main layout chunks
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(0),    // Main Content
                    Constraint::Length(3), // Footer
                ])
                .split(size);

            // Header widget
            let header = Paragraph::new(format!(" HackRF One | FW: {} ", fw_version))
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            f.render_widget(header, chunks[0]);

            // Main content area
            let main_block = Block::default()
                .title(" Device Status ")
                .borders(Borders::ALL);
            
            let info_text = format!(
                "Board ID: {:?}\nStatus: Monitoring...",
                board_id
            );
            
            let body = Paragraph::new(info_text)
                .block(main_block)
                .alignment(Alignment::Left);
            f.render_widget(body, chunks[1]);

            // Footer widget
            let footer = Paragraph::new(" Press 'q' to quit | 'r' to reset stats ")
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            f.render_widget(footer, chunks[2]);
        })?;

        // Poll for user input events
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    return Ok(());
                }
            }
        }
    }
}