mod hardware;
mod state;
use state::{
    SdrMetrics, DEFAULT_FREQUENCY, DEFAULT_LNA_GAIN, DEFAULT_SAMPLE_RATE, DEFAULT_VGA_GAIN,
    THROUGHPUT_HISTORY_LEN,
};

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline, Wrap},
    Terminal,
};
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;



#[tokio::main]
async fn main() -> Result<()> {
    let board = match hardware::Device::open() {
        Ok(b) => Arc::new(b),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let metrics = Arc::new(Mutex::new(SdrMetrics {
        frequency: DEFAULT_FREQUENCY,
        config_sample_rate: DEFAULT_SAMPLE_RATE,
        actual_sample_rate: 0,
        lna_gain: DEFAULT_LNA_GAIN,
        vga_gain: DEFAULT_VGA_GAIN,
        amp_enabled: false,
        rx_enabled: false,
        hw_streaming: false,
        bytes_since_last_poll: 0,
        last_poll_time: std::time::Instant::now(),
        current_throughput_bps: 0,
        throughput_history: VecDeque::with_capacity(THROUGHPUT_HISTORY_LEN),
        log: VecDeque::new(),
    }));

    let metrics_bg = Arc::clone(&metrics);
    let board_bg = Arc::clone(&board);
    tokio::spawn(async move {
        // Tracks whether we have actually issued start_rx to the hardware
        let mut hw_rx_active = false;

        loop {
            let now = std::time::Instant::now();

            // Compute throughput from bytes accumulated by the RX callback
            let (bytes, elapsed_ms) = {
                let mut m = metrics_bg.lock().unwrap();
                let elapsed = now.duration_since(m.last_poll_time).as_millis();
                let bytes = m.bytes_since_last_poll;
                m.bytes_since_last_poll = 0;
                m.last_poll_time = now;
                (bytes, elapsed)
            };

            {
                let mut m = metrics_bg.lock().unwrap();
                m.hw_streaming = board_bg.is_streaming();
                if elapsed_ms > 0 {
                    m.current_throughput_bps = (bytes * 1000) / elapsed_ms as u64;
                    // 2 bytes per IQ sample (8-bit I + 8-bit Q)
                    m.actual_sample_rate = (m.current_throughput_bps / 2) as u32;
                    // Record KB/s in history for sparkline
                    let throughput_kb = m.current_throughput_bps / 1024;
                    if m.throughput_history.len() >= THROUGHPUT_HISTORY_LEN {
                        m.throughput_history.pop_front();
                    }
                    m.throughput_history.push_back(throughput_kb);
                }
            }

            // Manage RX streaming based on user's desired state
            let rx_enabled = metrics_bg.lock().unwrap().rx_enabled;
            if rx_enabled && !hw_rx_active {
                let user_param = Arc::as_ptr(&metrics_bg) as *mut libc::c_void;
                match board_bg.start_rx(hardware::device::rx_callback, user_param) {
                    Ok(()) => {
                        hw_rx_active = true;
                        metrics_bg.lock().unwrap().push_log("RX streaming started");
                    }
                    Err(e) => {
                        let msg = format!("Error starting RX: {}", e);
                        let mut m = metrics_bg.lock().unwrap();
                        m.rx_enabled = false;
                        m.push_log(msg);
                    }
                }
            } else if !rx_enabled && hw_rx_active {
                let result = board_bg.stop_rx();
                hw_rx_active = false;
                let mut m = metrics_bg.lock().unwrap();
                match result {
                    Ok(()) => m.push_log("RX streaming stopped"),
                    Err(e) => m.push_log(format!("Error stopping RX: {}", e)),
                }
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });

    let version = board.version()?;
    let board_id = board.board_id()?;
    let board_name = board.board_name(board_id);
    let serial = board.serial_number()?;

    {
        let mut m = metrics.lock().unwrap();
        m.push_log(format!("Connected: {} | Serial: {}", board_name, serial));
        m.push_log(format!("Firmware: {}", version));
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, &version, &board_name, &serial, metrics);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Application error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    fw_version: &str,
    board_name: &str,
    serial: &str,
    metrics: Arc<Mutex<SdrMetrics>>,
) -> io::Result<()> {
    loop {
        let m = metrics.lock().unwrap().clone();

        terminal.draw(|f| {
            let size = f.size();

            // Outer vertical layout: header / body / log / footer
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Header
                    Constraint::Min(0),     // Body
                    Constraint::Length(7),  // Log panel
                    Constraint::Length(3),  // Footer
                ])
                .split(size);

            // Header: device name, firmware, serial
            let header = Paragraph::new(format!(
                " {} | FW: {} | S/N: {} ",
                board_name, fw_version, serial
            ))
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);
            f.render_widget(header, chunks[0]);

            // Body: telemetry (left) | gains + sparkline (right)
            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            let status_text = if m.hw_streaming { "STREAMING" } else { "IDLE" };
            let status_color = if m.hw_streaming { Color::Green } else { Color::Yellow };

            let info_text = format!(
                "Model:       {}\n\
                 Serial:      {}\n\
                 Status:      {}\n\n\
                 Frequency:   {:.3} MHz\n\
                 Sample Rate: {:.1} Msps (cfg)\n\
                 Throughput:  {:.2} MB/s ({:.1} Msps actual)\n\
                 AMP:         {}",
                board_name,
                serial,
                status_text,
                m.frequency as f64 / 1_000_000.0,
                m.config_sample_rate / 1_000_000.0,
                m.current_throughput_bps as f64 / (1024.0 * 1024.0),
                m.actual_sample_rate as f64 / 1_000_000.0,
                if m.amp_enabled { "ON" } else { "OFF" },
            );

            let telemetry_panel = Paragraph::new(info_text)
                .block(
                    Block::default()
                        .title(" Telemetry ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(status_color)),
                )
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true });
            f.render_widget(telemetry_panel, body_chunks[0]);

            // Right side: LNA / VGA / Sample Rate gauges + sparkline
            let gain_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // LNA gauge
                    Constraint::Length(3),  // VGA gauge
                    Constraint::Length(3),  // Sample Rate gauge
                    Constraint::Min(0),     // USB throughput sparkline
                ])
                .split(body_chunks[1]);

            let lna_gauge = Gauge::default()
                .block(
                    Block::default()
                        .title(format!(" LNA Gain: {} dB ", m.lna_gain))
                        .borders(Borders::ALL),
                )
                .gauge_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(Color::Black)
                        .add_modifier(Modifier::ITALIC),
                )
                // LNA valid range: 0–40 dB in 8 dB steps
                .percent(((m.lna_gain as f32 / 40.0) * 100.0) as u16);
            f.render_widget(lna_gauge, gain_chunks[0]);

            let vga_gauge = Gauge::default()
                .block(
                    Block::default()
                        .title(format!(" VGA Gain: {} dB ", m.vga_gain))
                        .borders(Borders::ALL),
                )
                .gauge_style(
                    Style::default()
                        .fg(Color::Magenta)
                        .bg(Color::Black)
                        .add_modifier(Modifier::ITALIC),
                )
                // VGA valid range: 0–62 dB in 2 dB steps
                .percent(((m.vga_gain as f32 / 62.0) * 100.0) as u16);
            f.render_widget(vga_gauge, gain_chunks[1]);

            let sr_gauge = Gauge::default()
                .block(
                    Block::default()
                        .title(format!(
                            " Sample Rate: {:.1} Msps ",
                            m.actual_sample_rate as f64 / 1_000_000.0
                        ))
                        .borders(Borders::ALL),
                )
                .gauge_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Black)
                        .add_modifier(Modifier::ITALIC),
                )
                // HackRF One max: 20 Msps
                .percent(((m.actual_sample_rate as f32 / 20_000_000.0) * 100.0).min(100.0) as u16);
            f.render_widget(sr_gauge, gain_chunks[2]);

            let sparkline_data: Vec<u64> = m.throughput_history.iter().cloned().collect();
            let sparkline_max = sparkline_data.iter().cloned().max().unwrap_or(0).max(1);
            let sparkline = Sparkline::default()
                .block(
                    Block::default()
                        .title(format!(" USB Throughput (KB/s, peak: {}) ", sparkline_max))
                        .borders(Borders::ALL),
                )
                .data(&sparkline_data)
                .max(sparkline_max)
                .style(Style::default().fg(Color::Green));
            f.render_widget(sparkline, gain_chunks[3]);

            // Log panel: most recent messages, oldest at top
            let log_lines: Vec<&str> = m.log.iter().map(|s| s.as_str()).collect();
            let log_text = log_lines.join("\n");
            let log_panel = Paragraph::new(log_text)
                .block(Block::default().title(" Log ").borders(Borders::ALL))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true });
            f.render_widget(log_panel, chunks[2]);

            let footer = Paragraph::new(
                " [Q] Quit | [SPACE] Start/Stop RX | [R] Reset to defaults ",
            )
            .block(Block::default().borders(Borders::ALL))
            .alignment(Alignment::Center);
            f.render_widget(footer, chunks[3]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char(' ') => {
                        let mut m = metrics.lock().unwrap();
                        m.rx_enabled = !m.rx_enabled;
                    }
                    KeyCode::Char('r') => {
                        metrics.lock().unwrap().reset_to_defaults();
                    }
                    _ => {}
                }
            }
        }
    }
}
