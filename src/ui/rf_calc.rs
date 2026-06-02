//! Shared RF-chain calculations: cascade noise figure, minimum detectable
//! signal, gain-staging advice, and ADC utilisation. Used by both the `rf_chain`
//! lab panel and the `micro_gain` field view so the numbers stay identical.

/// Cascade Noise Figure via Friis formula (result in dB).
///
/// HackRF One stage approximations:
///   AMP  — MGA-81563 front-end LNA: gain 14 dB, NF ~2.0 dB
///   LNA  — MAX2837 LNA: NF ~3.5 dB at max gain (40 dB), degrades ~0.15 dB
///          per dB of gain reduction (model: NF_LNA = 3.5 + (40−G)×0.15)
///   VGA  — MAX2837 baseband VGA: NF ~10 dB (contribution negligible at high LNA gain)
///
/// Friis: F_total = F₁ + (F₂−1)/G₁ + (F₃−1)/(G₁·G₂)  (all linear, → back to dB)
/// VGA gain is not a parameter — in a 3-stage cascade it does not appear in the
/// formula (there is no 4th stage whose noise it would need to suppress).
pub fn estimate_nf_db(amp_enabled: bool, lna_gain: u32) -> f64 {
    let lin = |db: f64| 10f64.powf(db / 10.0);

    let nf_lna = 3.5 + (40.0 - lna_gain as f64).max(0.0) * 0.15;
    let f_lna  = lin(nf_lna);
    let g_lna  = lin(lna_gain as f64);
    let f_vga  = lin(10.0);

    let f_total = if amp_enabled {
        let f_amp = lin(2.0);
        let g_amp = lin(14.0);
        f_amp + (f_lna - 1.0) / g_amp + (f_vga - 1.0) / (g_amp * g_lna)
    } else {
        f_lna + (f_vga - 1.0) / g_lna
    };

    10.0 * f_total.log10()
}

/// Minimum Detectable Signal in dBm.
///
/// MDS = kTB + NF  where kT = −174 dBm/Hz at 290 K.
/// Returns None when the BB filter bandwidth is unknown (0 Hz).
pub fn estimate_mds_dbm(bb_filter_hz: u32, nf_db: f64) -> Option<f64> {
    if bb_filter_hz == 0 { return None; }
    Some(-174.0 + 10.0 * (bb_filter_hz as f64).log10() + nf_db)
}

/// Gain-staging advice from the IQ amplitude histogram.
/// Returns `(text, severity)` where severity: 0 = OK, 1 = warn, 2 = crit.
pub fn gain_advice(hist: &[u64; 32]) -> (&'static str, u8) {
    let total: u64 = hist.iter().sum();
    if total == 0 { return ("no signal — start RX", 0); }
    let low:  u64 = hist[..8].iter().sum();
    let high: u64 = hist[24..].iter().sum();
    let low_pct  = low  * 100 / total;
    let high_pct = high * 100 / total;
    if high_pct > 10 {
        ("⬇ clipping — reduce gain", 2)
    } else if low_pct > 90 {
        ("⬆ weak — increase LNA +8 dB", 1)
    } else if low_pct > 70 {
        ("⬆ under-utilised — try +8 dB", 1)
    } else {
        ("✓ gain staging OK", 0)
    }
}

/// ADC utilisation: fraction of samples in the mid-range bins (8–23) of the IQ
/// amplitude histogram. 0 when there is no data.
pub fn adc_utilisation_ratio(hist: &[u64; 32]) -> f64 {
    let total: u64 = hist.iter().sum();
    let mid: u64   = hist[8..24].iter().sum();
    if total > 0 { mid as f64 / total as f64 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nf_amp_on_max_gain_is_near_amp_nf() {
        let nf = estimate_nf_db(true, 40);
        assert!(nf > 2.0 && nf < 3.0, "expected ~2.1 dB, got {:.2}", nf);
    }

    #[test]
    fn nf_amp_off_max_lna_gain_near_lna_nf() {
        let nf = estimate_nf_db(false, 40);
        assert!(nf > 3.4 && nf < 4.0, "expected ~3.5 dB, got {:.2}", nf);
    }

    #[test]
    fn nf_degrades_at_lower_lna_gain() {
        let nf_high = estimate_nf_db(false, 40);
        let nf_low  = estimate_nf_db(false,  8);
        assert!(nf_low > nf_high, "NF should be worse at lower LNA gain");
    }

    #[test]
    fn nf_amp_lowers_cascade_nf() {
        let nf_no_amp = estimate_nf_db(false, 24);
        let nf_amp    = estimate_nf_db(true,  24);
        assert!(nf_amp < nf_no_amp, "AMP should improve cascade NF");
    }

    #[test]
    fn gain_advice_clipping_is_crit() {
        let mut hist = [0u64; 32];
        hist[24] = 20; hist[0] = 80; // >10% in high bins
        let (_, sev) = gain_advice(&hist);
        assert_eq!(sev, 2);
    }

    #[test]
    fn gain_advice_weak_is_warn() {
        let mut hist = [0u64; 32];
        hist[0] = 95; hist[8] = 5; // >90% in low bins
        let (_, sev) = gain_advice(&hist);
        assert_eq!(sev, 1);
    }

    #[test]
    fn gain_advice_ok_is_zero() {
        let mut hist = [0u64; 32];
        hist[8] = 50; hist[16] = 50; // mid-range utilisation
        let (_, sev) = gain_advice(&hist);
        assert_eq!(sev, 0);
    }

    #[test]
    fn mds_none_when_bb_filter_zero() {
        assert!(estimate_mds_dbm(0, 3.5).is_none());
    }

    #[test]
    fn mds_10mhz_3_5db_nf() {
        // MDS = -174 + 10*log10(10_000_000) + 3.5 = -174 + 70 + 3.5 = -100.5 dBm
        let mds = estimate_mds_dbm(10_000_000, 3.5).unwrap();
        assert!((mds - (-100.5)).abs() < 0.1, "expected ~-100.5 dBm, got {:.1}", mds);
    }

    #[test]
    fn mds_improves_with_narrower_bw() {
        let mds_wide   = estimate_mds_dbm(10_000_000, 3.5).unwrap();
        let mds_narrow = estimate_mds_dbm( 5_000_000, 3.5).unwrap();
        assert!((mds_wide - mds_narrow - 3.0).abs() < 0.1,
            "halving BW should improve MDS by 3 dB");
    }

    #[test]
    fn adc_util_zero_when_empty() {
        assert_eq!(adc_utilisation_ratio(&[0u64; 32]), 0.0);
    }

    #[test]
    fn adc_util_counts_mid_bins() {
        let mut hist = [0u64; 32];
        hist[0] = 50;   // low (out of mid range)
        hist[16] = 50;  // mid
        assert!((adc_utilisation_ratio(&hist) - 0.5).abs() < 1e-9);
    }
}
