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

/// Minimal FFI wrapper for libhackrf to bypass the broken 0.0.1 crate
mod hackrf_ffi {
    use std::ffi::CStr;
    use libc::{c_int, c_char, c_void};

    extern "C" {
        fn hackrf_init() -> c_int;
        fn hackrf_exit() -> c_int;
        fn hackrf_open(device: *mut *mut c_void) -> c_int;
        fn hackrf_close(device: *mut c_void) -> c_int;
        fn hackrf_version_string_read(device: *mut c_void, version: *mut c_char, length: u8) -> c_int;
        fn hackrf_board_id_read(device: *mut c_void, value: *mut u8) -> c_int;
        fn hackrf_board_id_name(id: u8) -> *const c_char;
    }

    pub struct Device(*mut c_void);

    impl Device {
        pub fn open() -> anyhow::Result<Self> {
            unsafe {
                if hackrf_init() != 0 {
                    anyhow::bail!("Failed to initialize libhackrf");
                }
                let mut ptr = std::ptr::null_mut();
                if hackrf_open(&mut ptr) != 0 {
                    hackrf_exit();
                    anyhow::bail!("No HackRF device found or permission denied");
                }
                Ok(Device(ptr))
            }
        }

        pub fn version(&self) -> anyhow::Result<String> {
            let mut buf = [0; 64];
            unsafe {
                if self.0.is_null() { anyhow::bail!("Device pointer is null"); }
                if hackrf_version_string_read(self.0, buf.as_mut_ptr() as *mut c_char, 63) != 0 {
                    anyhow::bail!("Failed to read version");
                }
                Ok(CStr::from_ptr(buf.as_ptr() as *const c_char).to_string_lossy().into_owned())
            }
        }

        pub fn board_name(&self, id: u8) -> String {
            unsafe {
                let name_ptr = hackrf_board_id_name(id);
                if name_ptr.is_null() { return "Unknown".to_string(); }
                CStr::from_ptr(name_ptr).to_string_lossy().into_owned()
            }
        }

        pub fn board_id(&self) -> anyhow::Result<u8> {
            let mut id = 0u8;
            unsafe {
                if self.0.is_null() { anyhow::bail!("Device pointer is null"); }
                if hackrf_board_id_read(self.0, &mut id) != 0 {
                    anyhow::bail!("Failed to read board ID");
                }
                Ok(id)
            }
        }
    }

    impl Drop for Device {
        fn drop(&mut self) {
            unsafe {
                hackrf_close(self.0);
                hackrf_exit();
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Attempt to open the first available device using our custom FFI wrapper
    let board = match hackrf_ffi::Device::open() {
        Ok(b) => b,
        Err(e) => {
            return Err(anyhow::anyhow!("HackRF device not found or permission denied: {}", e));
        }
    };

    // Retrieve basic hardware information
    let version = board.version()?;
    let board_id = board.board_id()?;
    let board_name = board.board_name(board_id);

    // Setup terminal for TUI
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main application loop
    let app_result = run_app(&mut terminal, &version, &board_name);

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
    board_name: &str,
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
                "Model: {}\nStatus: Monitoring...",
                board_name
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