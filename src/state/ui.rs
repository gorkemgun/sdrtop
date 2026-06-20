use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::MicroView;

pub const LOG_MAX_ENTRIES: usize = 100;

/// Which lead view the Command Rail's mode-card shows. It auto-follows what you
/// are doing — tuning relaxes to **Hunt** (find signals), gain changes to
/// **Bench** (set up the chain) — and falls back to **Monitor** when idle. `Tab`
/// in rail-focus pins a mode manually (no auto-decay).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RailMode {
    Hunt,
    #[default]
    Monitor,
    Bench,
}

impl RailMode {
    /// The full-width tab label.
    pub fn label(self) -> &'static str {
        match self {
            RailMode::Hunt    => "HUNT",
            RailMode::Monitor => "MONITOR",
            RailMode::Bench   => "BENCH",
        }
    }

    /// Left-to-right tab order, also the manual `Tab` cycle order.
    pub const ALL: [RailMode; 3] = [RailMode::Hunt, RailMode::Monitor, RailMode::Bench];

    /// Next mode in the `Tab` cycle: HUNT → MONITOR → BENCH → HUNT.
    pub fn next(self) -> RailMode {
        match self {
            RailMode::Hunt    => RailMode::Monitor,
            RailMode::Monitor => RailMode::Bench,
            RailMode::Bench   => RailMode::Hunt,
        }
    }
}

/// How long an auto-set mode (Hunt/Bench) lingers before relaxing back to
/// Monitor.
pub const RAIL_MODE_DECAY: Duration = Duration::from_secs(8);

/// The mode to actually render, after decay: an auto-set Hunt/Bench falls back
/// to Monitor once it's been idle past [`RAIL_MODE_DECAY`]; a manually-pinned
/// mode (`auto == false`) never decays. Pure so it's testable without a clock.
pub fn decayed_mode(mode: RailMode, auto: bool, since: Option<Duration>) -> RailMode {
    match (auto, since) {
        (true, Some(d)) if d >= RAIL_MODE_DECAY => RailMode::Monitor,
        _ => mode,
    }
}

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
    /// Command Rail lead-view mode (Hunt/Monitor/Bench). See [`RailMode`].
    pub rail_mode:              RailMode,
    /// Whether `rail_mode` was set by auto-follow (decays) vs. pinned by `Tab`.
    pub rail_mode_auto:         bool,
    /// When the last auto mode-change happened, for the idle decay to Monitor.
    pub last_mode_action:       Option<Instant>,
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

    /// Record an auto-follow mode change (tuning → Hunt, gain → Bench): set the
    /// mode and (re)start its idle decay timer.
    pub fn note_mode_action(&mut self, mode: RailMode) {
        self.rail_mode        = mode;
        self.rail_mode_auto   = true;
        self.last_mode_action = Some(Instant::now());
    }

    /// Manual `Tab` cycle in rail-focus: pin the next mode so it won't decay.
    pub fn cycle_rail_mode(&mut self) -> RailMode {
        self.rail_mode        = self.rail_mode.next();
        self.rail_mode_auto   = false;
        self.last_mode_action = None;
        self.rail_mode
    }

    /// The mode to render right now, after applying the idle decay.
    pub fn effective_rail_mode(&self) -> RailMode {
        decayed_mode(self.rail_mode, self.rail_mode_auto,
                     self.last_mode_action.map(|t| t.elapsed()))
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
            rail_mode:              RailMode::default(),
            rail_mode_auto:         false,
            last_mode_action:       None,
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
    fn rail_mode_cycles_hunt_monitor_bench() {
        assert_eq!(RailMode::Hunt.next(), RailMode::Monitor);
        assert_eq!(RailMode::Monitor.next(), RailMode::Bench);
        assert_eq!(RailMode::Bench.next(), RailMode::Hunt);
        assert_eq!(RailMode::default(), RailMode::Monitor);
        assert_eq!(RailMode::ALL, [RailMode::Hunt, RailMode::Monitor, RailMode::Bench]);
    }

    #[test]
    fn auto_mode_decays_to_monitor_when_idle() {
        // Fresh auto-set Hunt holds; past the decay window it relaxes to Monitor.
        assert_eq!(decayed_mode(RailMode::Hunt, true, Some(Duration::from_secs(2))), RailMode::Hunt);
        assert_eq!(decayed_mode(RailMode::Hunt, true, Some(RAIL_MODE_DECAY)), RailMode::Monitor);
        // A manually-pinned mode never decays.
        assert_eq!(decayed_mode(RailMode::Bench, false, Some(Duration::from_secs(999))), RailMode::Bench);
        // No timer yet → whatever the mode is.
        assert_eq!(decayed_mode(RailMode::Bench, true, None), RailMode::Bench);
    }

    #[test]
    fn note_and_cycle_set_auto_flag() {
        let mut ui = UiState::default();
        ui.note_mode_action(RailMode::Hunt);
        assert_eq!(ui.rail_mode, RailMode::Hunt);
        assert!(ui.rail_mode_auto && ui.last_mode_action.is_some());
        // Tab pins the next mode and clears the decay timer.
        assert_eq!(ui.cycle_rail_mode(), RailMode::Monitor);
        assert!(!ui.rail_mode_auto && ui.last_mode_action.is_none());
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
