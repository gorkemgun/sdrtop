use std::collections::VecDeque;

pub const THROUGHPUT_HISTORY_LEN: usize = 64;
pub const LOG_MAX_ENTRIES: usize = 100;

pub const DEFAULT_LNA_GAIN: u32 = 16;
pub const DEFAULT_VGA_GAIN: u32 = 20;
pub const DEFAULT_FREQUENCY: u64 = 2_400_000_000;
pub const DEFAULT_SAMPLE_RATE: f64 = 10_000_000.0;

#[derive(Clone)]
pub struct SdrMetrics {
    pub frequency: u64,
    pub config_sample_rate: f64,
    pub actual_sample_rate: u32,
    pub lna_gain: u32,
    pub vga_gain: u32,
    pub amp_enabled: bool,
    // User-desired RX state (toggled by Space); separate from hw_streaming
    pub rx_enabled: bool,
    // Actual hardware streaming state, updated by the polling task
    pub hw_streaming: bool,
    pub bytes_since_last_poll: u64,
    pub last_poll_time: std::time::Instant,
    pub current_throughput_bps: u64,
    // Throughput history in KB/s for sparkline display
    pub throughput_history: VecDeque<u64>,
    // In-app log messages (replaces eprintln! while TUI is active)
    pub log: VecDeque<String>,
}

impl SdrMetrics {
    pub fn push_log(&mut self, msg: impl Into<String>) {
        if self.log.len() >= LOG_MAX_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(msg.into());
    }

    pub fn reset_to_defaults(&mut self) {
        self.lna_gain = DEFAULT_LNA_GAIN;
        self.vga_gain = DEFAULT_VGA_GAIN;
        self.amp_enabled = false;
        self.frequency = DEFAULT_FREQUENCY;
        self.config_sample_rate = DEFAULT_SAMPLE_RATE;
        self.push_log("Settings reset to defaults");
    }
}
