//! Lab "instrument mode" measurement state — the REF / averaging / CAL settings
//! the lab presets' instrument-chrome (banner + marker bar) shows. The marker
//! data itself lives in [`SpectrumState`](super::SpectrumState) (not duplicated
//! here); this struct is the home for the measurement-state flags.
//!
//! Most fields are placeholders for a later step (peak-hold / averaging / ref-
//! trace wiring) — until then they format as `—`/`OFF` so the banner renders
//! honestly rather than faking values.

/// Measurement-state for the lab instrument-chrome banner.
#[derive(Clone)]
pub struct LabState {
    /// Reference level (dBFS) for the banner's `REF` field. `None` → `—`.
    pub ref_dbfs: Option<f32>,
    /// Spectrum averaging depth; `1` means no averaging (`OFF`).
    pub avg_n:    u16,
    /// Whether a calibration reference is active.
    pub cal:      bool,
}

impl Default for LabState {
    fn default() -> Self {
        Self { ref_dbfs: None, avg_n: 1, cal: false }
    }
}

impl LabState {
    /// `REF` banner field: e.g. `-10 dBFS`, or `—` when unset.
    pub fn ref_label(&self) -> String {
        match self.ref_dbfs {
            Some(db) => format!("{db:.0} dBFS"),
            None     => "—".to_string(),
        }
    }

    /// `AVG` banner field: `×8` when averaging, else `OFF`.
    pub fn avg_label(&self) -> String {
        if self.avg_n > 1 { format!("\u{00D7}{}", self.avg_n) } else { "OFF".to_string() }
    }

    /// `CAL` banner field: `✓` when calibrated, else `—`.
    pub fn cal_label(&self) -> &'static str {
        if self.cal { "\u{2713}" } else { "\u{2014}" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_reads_as_unset() {
        let s = LabState::default();
        assert_eq!(s.ref_label(), "\u{2014}"); // —
        assert_eq!(s.avg_label(), "OFF");
        assert_eq!(s.cal_label(), "\u{2014}"); // —
    }

    #[test]
    fn populated_labels_format() {
        let s = LabState { ref_dbfs: Some(-10.0), avg_n: 8, cal: true };
        assert_eq!(s.ref_label(), "-10 dBFS");
        assert_eq!(s.avg_label(), "\u{00D7}8"); // ×8
        assert_eq!(s.cal_label(), "\u{2713}");  // ✓
    }
}
