use std::sync::{Arc, Mutex};

use crossbeam_channel::Receiver;
use num_complex::Complex;
use rustfft::FftPlanner;

use crate::dsp::{self, WindowFn};
use crate::state::{FftFrame, SdrMetrics};

const DB_FLOOR: f32 = -160.0;

pub struct FftWorker {
    pub sample_rx: Receiver<Vec<u8>>,
    pub state: Arc<Mutex<SdrMetrics>>,
    pub fft_size: usize,
    pub window_fn: WindowFn,
    pub ema_alpha: f32,
    pub peak_decay_db: f32,
}

impl FftWorker {
    pub fn new(sample_rx: Receiver<Vec<u8>>, state: Arc<Mutex<SdrMetrics>>) -> Self {
        Self {
            sample_rx,
            state,
            fft_size: 2048,
            window_fn: WindowFn::Hann,
            ema_alpha: 0.2,
            peak_decay_db: 0.5,
        }
    }

    pub fn run(self) {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(self.fft_size);
        let window = dsp::compute_window(self.window_fn, self.fft_size);

        let mut buf: Vec<u8> = Vec::new();
        let mut smoothed: Vec<f32> = Vec::new();
        let mut peak: Vec<f32> = Vec::new();

        while let Ok(chunk) = self.sample_rx.recv() {
            buf.extend_from_slice(&chunk);

            while buf.len() >= self.fft_size * 2 {
                // Convert to windowed complex samples
                let mut samples: Vec<Complex<f32>> = buf[..self.fft_size * 2]
                    .chunks_exact(2)
                    .zip(window.iter())
                    .map(|(pair, &w)| Complex {
                        re: pair[0] as i8 as f32 / 128.0 * w,
                        im: pair[1] as i8 as f32 / 128.0 * w,
                    })
                    .collect();
                buf.drain(..self.fft_size * 2);

                fft.process(&mut samples);

                // Magnitude → dBFS; normalize by fft_size for size-independent scale
                let mags: Vec<f32> = samples
                    .iter()
                    .map(|z| {
                        let norm = z.norm() / self.fft_size as f32;
                        if norm > 0.0 { 20.0 * norm.log10() } else { DB_FLOOR }
                    })
                    .collect();

                // fftshift: rotate by N/2 so DC lands at center
                let n = mags.len();
                let mut shifted = Vec::with_capacity(n);
                shifted.extend_from_slice(&mags[n / 2..]);
                shifted.extend_from_slice(&mags[..n / 2]);

                // EMA smoothing
                if smoothed.is_empty() {
                    smoothed = shifted.clone();
                } else {
                    let alpha = self.ema_alpha;
                    for (s, &new) in smoothed.iter_mut().zip(shifted.iter()) {
                        *s = alpha * new + (1.0 - alpha) * *s;
                    }
                }

                // Peak hold with per-frame decay
                if peak.is_empty() {
                    peak = smoothed.clone();
                } else {
                    let decay = self.peak_decay_db;
                    for (p, &s) in peak.iter_mut().zip(smoothed.iter()) {
                        *p = (*p - decay).max(s);
                    }
                }

                // Noise floor: mean of bottom 10% of bins
                let mut sorted = smoothed.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let count = (sorted.len() / 10).max(1);
                let noise_floor = sorted[..count].iter().sum::<f32>() / count as f32;

                // Read freq/rate while unlocked from the FFT loop
                let (center_freq_hz, sample_rate) = self
                    .state
                    .lock()
                    .map(|m| (m.frequency, m.config_sample_rate))
                    .unwrap_or((0, 0.0));

                if let Ok(mut m) = self.state.lock() {
                    m.last_fft_frame = Some(FftFrame {
                        bins_dbfs: smoothed.clone(),
                        peak_hold: peak.clone(),
                        noise_floor,
                        center_freq_hz,
                        sample_rate,
                        timestamp: std::time::Instant::now(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fftshift_dc_at_center() {
        let n = 8usize;
        let mags: Vec<f32> = (0..n).map(|i| i as f32).collect();
        let mut shifted = Vec::with_capacity(n);
        shifted.extend_from_slice(&mags[n / 2..]);
        shifted.extend_from_slice(&mags[..n / 2]);
        // shifted = [4,5,6,7,0,1,2,3]; DC (was 0) is now at index 4 = N/2
        assert_eq!(shifted[n / 2], 0.0, "DC should be at index N/2 after shift");
        assert_eq!(shifted[0], 4.0);
    }

    #[test]
    fn magnitude_floor_for_zero_input() {
        let z = Complex { re: 0.0f32, im: 0.0f32 };
        let norm = z.norm() / 2048.0f32;
        let db = if norm > 0.0 { 20.0 * norm.log10() } else { DB_FLOOR };
        assert_eq!(db, DB_FLOOR);
    }

    #[test]
    fn iq_byte_i8_max_converts_correctly() {
        let byte: u8 = 0x7F;
        let f = byte as i8 as f32 / 128.0;
        assert!((f - 0.9921875).abs() < 1e-6, "got {}", f);
    }

    #[test]
    fn iq_byte_i8_min_converts_correctly() {
        let byte: u8 = 0x80;
        let f = byte as i8 as f32 / 128.0;
        assert!((f - (-1.0)).abs() < 1e-6, "got {}", f);
    }

    #[test]
    fn ema_converges_to_new_value() {
        let mut s = 0.0f32;
        let target = 1.0f32;
        let alpha = 0.5f32;
        for _ in 0..20 {
            s = alpha * target + (1.0 - alpha) * s;
        }
        assert!(s > 0.99, "EMA should converge to target, got {}", s);
    }
}
