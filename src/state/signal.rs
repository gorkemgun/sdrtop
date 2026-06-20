use std::collections::VecDeque;

#[derive(Clone)]
pub struct SignalState {
    pub drops_per_sec:       u64,
    pub total_drops_session: u64,
    pub drop_history:        VecDeque<u64>,
    pub adc_saturation_pct:  f32,
    pub adc_saturation_peak: f32,
    pub saturation_history:  VecDeque<f32>,
    pub peak_to_nf_db:       f32,
    pub channel_power_dbfs:  f32,
    pub occupied_bw_hz:      u64,
    pub usb_errors_session:   u64,
    pub usb_errors_last_poll: u64,
    pub usb_error_history:    std::collections::VecDeque<u64>,
    /// Recent SNR (peak/noise-floor) samples, pushed by the rx poll task roughly
    /// every 500 ms while streaming. Powers the micro_signal trend arrow.
    pub snr_history:          VecDeque<f32>,
    /// Recent channel-power (dBFS) samples — pushed alongside `snr_history` at the
    /// same ~500 ms cadence. Powers the command rail's PWR sparkline + trend.
    pub pwr_history:          VecDeque<f32>,
    /// Recent noise-floor (dBFS) samples — pushed alongside `snr_history`. Powers
    /// the command rail's NF sparkline + trend.
    pub nf_history:           VecDeque<f32>,
}

impl SignalState {
    /// Short-term SNR trend in dB: mean of the most recent half of
    /// `snr_history` minus the mean of the older half. Positive means the
    /// signal is strengthening. `None` until there are enough samples.
    pub fn snr_delta(&self) -> Option<f32> {
        let n = self.snr_history.len();
        if n < 4 { return None; }
        let half = n / 2;
        let older:  f32 = self.snr_history.iter().take(half).sum::<f32>() / half as f32;
        let recent: f32 = self.snr_history.iter().skip(n - half).sum::<f32>() / half as f32;
        Some(recent - older)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_history(samples: &[f32]) -> SignalState {
        let mut s = SignalState {
            drops_per_sec: 0, total_drops_session: 0, drop_history: VecDeque::new(),
            adc_saturation_pct: 0.0, adc_saturation_peak: 0.0, saturation_history: VecDeque::new(),
            peak_to_nf_db: 0.0, channel_power_dbfs: 0.0, occupied_bw_hz: 0,
            usb_errors_session: 0, usb_errors_last_poll: 0, usb_error_history: VecDeque::new(),
            snr_history: VecDeque::new(), pwr_history: VecDeque::new(), nf_history: VecDeque::new(),
        };
        s.snr_history.extend(samples.iter().copied());
        s
    }

    #[test]
    fn snr_delta_none_with_too_few_samples() {
        assert_eq!(with_history(&[10.0, 12.0, 14.0]).snr_delta(), None);
    }

    #[test]
    fn snr_delta_positive_when_rising() {
        // older half avg = 10, recent half avg = 20 → +10
        let d = with_history(&[10.0, 10.0, 20.0, 20.0]).snr_delta().unwrap();
        assert!((d - 10.0).abs() < 1e-6, "got {d}");
    }

    #[test]
    fn snr_delta_negative_when_falling() {
        let d = with_history(&[20.0, 20.0, 12.0, 12.0]).snr_delta().unwrap();
        assert!((d + 8.0).abs() < 1e-6, "got {d}");
    }
}
