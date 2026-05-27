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

mod hackrf_ffi {
    use libc::{c_char, c_int, c_void};
    use std::ffi::CStr;

    #[repr(C)]
    pub struct hackrf_transfer {
        pub device: *mut c_void,
        pub buffer: *mut u8,
        pub buffer_length: i32,
        pub valid_length: i32,
        pub rx_ctx: *mut c_void,
        pub tx_ctx: *mut c_void,
    }

    // Matches hackrf_device_list_t in hackrf.h exactly
    #[repr(C)]
    pub struct HackrfDeviceList {
        pub serial_numbers: *mut *mut c_char,
        pub usb_board_ids: *mut c_int,
        pub usb_device_count: *mut c_int,
        pub usb_devices: *mut *mut c_void,
        pub usb_device_index: *mut c_int,
        pub devicecount: c_int,
    }

    #[repr(C)]
    pub struct ReadPartidSerialno {
        pub part_id: [u32; 2],
        pub serial_no: [u32; 4],
    }

    pub type HackrfTransferCallback = extern "C" fn(*mut hackrf_transfer) -> c_int;

    extern "C" {
        fn hackrf_init() -> c_int;
        fn hackrf_exit() -> c_int;
        fn hackrf_close(device: *mut c_void) -> c_int;
        fn hackrf_device_list() -> *mut HackrfDeviceList;
        fn hackrf_device_list_free(list: *mut HackrfDeviceList);
        fn hackrf_device_list_open(
            list: *mut HackrfDeviceList,
            index: c_int,
            device: *mut *mut c_void,
        ) -> c_int;
        fn hackrf_version_string_read(
            device: *mut c_void,
            version: *mut c_char,
            length: u8,
        ) -> c_int;
        fn hackrf_is_streaming(device: *mut c_void) -> c_int;
        fn hackrf_set_sample_rate(device: *mut c_void, freq_hz: f64) -> c_int;
        fn hackrf_set_freq(device: *mut c_void, freq_hz: u64) -> c_int;
        fn hackrf_set_amp_enable(device: *mut c_void, value: u8) -> c_int;
        fn hackrf_start_rx(
            device: *mut c_void,
            callback: HackrfTransferCallback,
            user_param: *mut c_void,
        ) -> c_int;
        fn hackrf_stop_rx(device: *mut c_void) -> c_int;
        fn hackrf_set_lna_gain(device: *mut c_void, value: u32) -> c_int;
        fn hackrf_set_vga_gain(device: *mut c_void, value: u32) -> c_int;
        fn hackrf_board_partid_serialno_read(
            device: *mut c_void,
            value: *mut ReadPartidSerialno,
        ) -> c_int;
        fn hackrf_board_id_read(device: *mut c_void, value: *mut u8) -> c_int;
        fn hackrf_board_id_name(id: u8) -> *const c_char;
        fn hackrf_error_name(errcode: c_int) -> *const c_char;
    }

    pub struct Device(*mut c_void);

    // Safety: libhackrf is thread-safe for status polling and streaming control
    unsafe impl Send for Device {}
    unsafe impl Sync for Device {}

    #[allow(dead_code)]
    impl Device {
        pub fn open() -> anyhow::Result<Self> {
            unsafe {
                let init_res = hackrf_init();
                if init_res != 0 {
                    let err = CStr::from_ptr(hackrf_error_name(init_res)).to_string_lossy();
                    anyhow::bail!("Failed to initialize libhackrf: {}", err);
                }

                let list_ptr = hackrf_device_list();
                if list_ptr.is_null() {
                    hackrf_exit();
                    anyhow::bail!("Failed to retrieve HackRF device list.");
                }

                let list = &*list_ptr;
                let count = list.devicecount as usize;

                if count == 0 {
                    hackrf_device_list_free(list_ptr);
                    hackrf_exit();
                    anyhow::bail!(
                        "No HackRF device found. Please connect your device and try again."
                    );
                }

                let selected_index = if count == 1 {
                    0
                } else {
                    println!("Multiple HackRF devices found:");
                    let mut valid_count = 0;
                    if !list.serial_numbers.is_null() {
                        for i in 0..count {
                            let serial_ptr = *list.serial_numbers.add(i);
                            if !serial_ptr.is_null() {
                                let serial = CStr::from_ptr(serial_ptr).to_string_lossy();
                                println!("[{}] Serial: {}", i, serial);
                                valid_count += 1;
                            }
                        }
                    }

                    if valid_count == 0 {
                        hackrf_device_list_free(list_ptr);
                        hackrf_exit();
                        anyhow::bail!("No valid serial numbers found for connected devices.");
                    }
                    print!("Select device index [0-{}]: ", count - 1);
                    use std::io::{self, Write};
                    io::stdout().flush()?;

                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let idx = input.trim().parse::<usize>().unwrap_or(usize::MAX);
                    if idx >= count {
                        hackrf_device_list_free(list_ptr);
                        hackrf_exit();
                        anyhow::bail!("Invalid device index selected.");
                    }
                    idx
                };

                let mut ptr = std::ptr::null_mut();
                let res =
                    hackrf_device_list_open(list_ptr, selected_index as c_int, &mut ptr);
                hackrf_device_list_free(list_ptr);

                if res != 0 {
                    let err = CStr::from_ptr(hackrf_error_name(res)).to_string_lossy();
                    hackrf_exit();
                    anyhow::bail!("Failed to open HackRF device: {} (code {})", err, res);
                }

                Ok(Device(ptr))
            }
        }

        pub fn version(&self) -> anyhow::Result<String> {
            let mut buf = [0i8; 64];
            unsafe {
                if hackrf_version_string_read(self.0, buf.as_mut_ptr(), 63) != 0 {
                    anyhow::bail!("Failed to read firmware version");
                }
                Ok(CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned())
            }
        }

        pub fn is_streaming(&self) -> bool {
            unsafe { hackrf_is_streaming(self.0) == 1 }
        }

        pub fn start_rx(
            &self,
            callback: HackrfTransferCallback,
            user_param: *mut libc::c_void,
        ) -> anyhow::Result<()> {
            unsafe {
                if hackrf_start_rx(self.0, callback, user_param) != 0 {
                    anyhow::bail!("Failed to start RX streaming");
                }
            }
            Ok(())
        }

        pub fn stop_rx(&self) -> anyhow::Result<()> {
            unsafe {
                if hackrf_stop_rx(self.0) != 0 {
                    anyhow::bail!("Failed to stop RX streaming");
                }
            }
            Ok(())
        }

        pub fn set_lna_gain(&self, gain: u32) -> anyhow::Result<()> {
            unsafe {
                if hackrf_set_lna_gain(self.0, gain) != 0 {
                    anyhow::bail!("Failed to set LNA gain");
                }
            }
            Ok(())
        }

        pub fn set_vga_gain(&self, gain: u32) -> anyhow::Result<()> {
            unsafe {
                if hackrf_set_vga_gain(self.0, gain) != 0 {
                    anyhow::bail!("Failed to set VGA gain");
                }
            }
            Ok(())
        }

        pub fn set_sample_rate(&self, sample_rate: f64) -> anyhow::Result<()> {
            unsafe {
                if hackrf_set_sample_rate(self.0, sample_rate) != 0 {
                    anyhow::bail!("Failed to set sample rate");
                }
            }
            Ok(())
        }

        pub fn set_frequency(&self, freq_hz: u64) -> anyhow::Result<()> {
            unsafe {
                if hackrf_set_freq(self.0, freq_hz) != 0 {
                    anyhow::bail!("Failed to set frequency");
                }
            }
            Ok(())
        }

        pub fn set_amp_enable(&self, enable: bool) -> anyhow::Result<()> {
            unsafe {
                if hackrf_set_amp_enable(self.0, enable as u8) != 0 {
                    anyhow::bail!("Failed to set AMP enable");
                }
            }
            Ok(())
        }

        pub fn board_id(&self) -> anyhow::Result<u8> {
            let mut id = 0u8;
            unsafe {
                if hackrf_board_id_read(self.0, &mut id) != 0 {
                    anyhow::bail!("Failed to read board ID");
                }
                Ok(id)
            }
        }

        pub fn board_name(&self, id: u8) -> String {
            unsafe {
                let ptr = hackrf_board_id_name(id);
                if ptr.is_null() {
                    return "Unknown".to_string();
                }
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }

        pub fn serial_number(&self) -> anyhow::Result<String> {
            let mut data = ReadPartidSerialno {
                part_id: [0; 2],
                serial_no: [0; 4],
            };
            unsafe {
                if hackrf_board_partid_serialno_read(self.0, &mut data) != 0 {
                    anyhow::bail!("Failed to read serial number");
                }
                let s = data.serial_no;
                Ok(format!(
                    "{:08x}{:08x}{:08x}{:08x}",
                    s[0], s[1], s[2], s[3]
                ))
            }
        }
    }

    impl Drop for Device {
        fn drop(&mut self) {
            unsafe {
                if hackrf_is_streaming(self.0) == 1 {
                    let _ = hackrf_stop_rx(self.0);
                }
                hackrf_close(self.0);
                hackrf_exit();
            }
        }
    }
}

extern "C" fn rx_callback(transfer: *mut hackrf_ffi::hackrf_transfer) -> libc::c_int {
    unsafe {
        let t = &*transfer;
        let metrics_ptr = t.rx_ctx as *const Mutex<SdrMetrics>;
        if !metrics_ptr.is_null() {
            if let Ok(mut m) = (*metrics_ptr).lock() {
                m.bytes_since_last_poll += t.valid_length as u64;
            }
        }
    }
    0
}

const THROUGHPUT_HISTORY_LEN: usize = 64;
const LOG_MAX_ENTRIES: usize = 100;

// Default gain/frequency values used on startup and on reset
const DEFAULT_LNA_GAIN: u32 = 16;
const DEFAULT_VGA_GAIN: u32 = 20;
const DEFAULT_FREQUENCY: u64 = 2_400_000_000;
const DEFAULT_SAMPLE_RATE: f64 = 10_000_000.0;

#[derive(Clone)]
struct SdrMetrics {
    frequency: u64,
    config_sample_rate: f64,
    actual_sample_rate: u32,
    lna_gain: u32,
    vga_gain: u32,
    amp_enabled: bool,
    // User-desired RX state (toggled by Space); separate from hw_streaming
    rx_enabled: bool,
    // Actual hardware streaming state, updated by the polling task
    hw_streaming: bool,
    bytes_since_last_poll: u64,
    last_poll_time: std::time::Instant,
    current_throughput_bps: u64,
    // Throughput history in KB/s for sparkline display
    throughput_history: VecDeque<u64>,
    // In-app log messages (replaces eprintln! while TUI is active)
    log: VecDeque<String>,
}

impl SdrMetrics {
    fn push_log(&mut self, msg: impl Into<String>) {
        if self.log.len() >= LOG_MAX_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(msg.into());
    }

    fn reset_to_defaults(&mut self) {
        self.lna_gain = DEFAULT_LNA_GAIN;
        self.vga_gain = DEFAULT_VGA_GAIN;
        self.amp_enabled = false;
        self.frequency = DEFAULT_FREQUENCY;
        self.config_sample_rate = DEFAULT_SAMPLE_RATE;
        self.push_log("Settings reset to defaults");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let board = match hackrf_ffi::Device::open() {
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
                match board_bg.start_rx(rx_callback, user_param) {
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
