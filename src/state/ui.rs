use std::collections::VecDeque;
use std::sync::Arc;

use super::MicroView;

pub const LOG_MAX_ENTRIES: usize = 100;

/// Severity of a log line, used by the log panel to pick a status lamp + colour.
/// Derived from the message text (see [`LogLevel::infer`]) so the ~86 existing
/// `push_log` call sites keep working unchanged.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogLevel { Info, Ok, Warn, Error }

impl LogLevel {
    /// Classify a message by keyword. Order matters: hard failures win over the
    /// "warning" prefix, which wins over success words. Everything else is Info.
    pub fn infer(msg: &str) -> LogLevel {
        let m = msg.to_ascii_lowercase();
        let has = |needles: &[&str]| needles.iter().any(|n| m.contains(n));
        if has(&["error", "fail", "unable", "in use", "could not"]) {
            LogLevel::Error
        } else if has(&["warning", "warn", "no marker", "unexpectedly"]) {
            LogLevel::Warn
        } else if has(&["started", "connected", "enabled", "success"]) {
            LogLevel::Ok
        } else {
            LogLevel::Info
        }
    }
}

/// One structured log line: when it happened (Unix epoch seconds, captured at
/// push time so the timestamp is fixed), its severity, and the message text.
#[derive(Clone)]
pub struct LogEntry {
    pub at_epoch_secs: u64,
    pub level:         LogLevel,
    pub text:          Arc<str>,
}

#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    FrequencyInput,
    SampleRateInput,
    MarkerNameInput,
    SweepStartInput,
    SweepStopInput,
}

#[derive(Clone)]
pub struct UiState {
    pub input_mode:             InputMode,
    pub input_buf:              String,
    pub focused_panel:          Option<String>,
    pub focused_panel_bindings: &'static [(&'static str, &'static str)],
    /// Name of the engine's active preset, synced each frame before draw so the
    /// footer can show it. The engine owns the authoritative value; this is a
    /// render-time mirror.
    pub active_preset:          String,
    /// Names of all defined presets, synced each frame alongside active_preset.
    /// Lets the footer build the lab map from presets that actually exist.
    pub preset_names:           Vec<String>,
    /// Current position in the micro `[0]` cycle. Advanced by the `[0]` handler;
    /// read by the footer to show "micro N/M".
    pub micro_view:             MicroView,
    pub log:                    VecDeque<LogEntry>,
}

impl UiState {
    pub fn push_log(&mut self, msg: impl Into<String>) {
        let text = msg.into();
        let level = LogLevel::infer(&text);
        let at_epoch_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if self.log.len() >= LOG_MAX_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(LogEntry { at_epoch_secs, level, text: Arc::from(text) });
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            input_mode:             InputMode::Normal,
            input_buf:              String::new(),
            focused_panel:          None,
            focused_panel_bindings: &[],
            active_preset:          String::new(),
            preset_names:           Vec::new(),
            micro_view:             MicroView::default(),
            log:                    VecDeque::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_flags_failures_as_error() {
        for msg in [
            "Tune error: out of range",
            "Error stopping RX: timeout",
            "Startup: failed to set gain",
            "Device is in use by another process",
            "Reset error: denied",
        ] {
            assert_eq!(LogLevel::infer(msg), LogLevel::Error, "{msg:?}");
        }
    }

    #[test]
    fn infer_flags_warnings() {
        for msg in [
            "WARNING: Streaming stopped unexpectedly — press [Space]",
            "No marker near cursor — place one with [M] first",
        ] {
            assert_eq!(LogLevel::infer(msg), LogLevel::Warn, "{msg:?}");
        }
    }

    #[test]
    fn infer_flags_success_events() {
        assert_eq!(LogLevel::infer("RX streaming started"), LogLevel::Ok);
        assert_eq!(LogLevel::infer("Connected: HackRF One | Serial: …"), LogLevel::Ok);
    }

    #[test]
    fn infer_defaults_to_info() {
        for msg in [
            "Step → 100 kHz",
            "Freq zoom: ×4",
            "Preset: spectrum",
            "RX streaming stopped",
        ] {
            assert_eq!(LogLevel::infer(msg), LogLevel::Info, "{msg:?}");
        }
    }

    #[test]
    fn push_log_captures_level_and_time() {
        let mut ui = UiState::default();
        ui.push_log("Tune error: nope");
        let e = ui.log.back().unwrap();
        assert_eq!(e.level, LogLevel::Error);
        assert_eq!(e.text.as_ref(), "Tune error: nope");
        assert!(e.at_epoch_secs > 0, "timestamp captured");
    }
}
