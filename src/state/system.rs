use std::collections::VecDeque;
use std::sync::Arc;

#[derive(Clone)]
pub struct SystemState {
    pub board_name:      Arc<str>,
    #[allow(dead_code)]
    pub serial:          Arc<str>,
    pub fw_version:      Arc<str>,
    pub board_rev:       u8,
    pub usb_api_version: u16,
    pub process_cpu_pct: f32,
    pub process_rss_mb:  u64,
    /// CPU % × 10 per sample (0.1 % resolution), one entry per system task poll (1 s).
    pub cpu_history:     VecDeque<u64>,
}
