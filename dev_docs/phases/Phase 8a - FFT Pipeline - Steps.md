# Phase 8a — FFT Pipeline: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)

**Goal:** Build the data pipeline from `rx_callback` → crossbeam channel →
`FftWorker` → `FftFrame` in `SdrMetrics`. Phase 8b wires the display.

---

## Correctness notes

**IQ byte conversion:** Cast to `i8` first, then `f32`:
`pair[0] as i8 as f32 / 128.0`. Never `pair[0] as f32 / 128.0 - 1.0` — the
`as f32` of a `u8` preserves the unsigned bit pattern; the offset trick gives wrong
results for values like `0x80` (`−128i8` would become `0.0` instead of `−1.0`).

**fftshift:** rustfft puts DC at bin 0, positive frequencies at 1..N/2, and
the negative-frequency alias at N/2..N. For a centered display, rotate by N/2:
new output = `[old[N/2..], old[..N/2]]`. After shift: bin 0 = lowest frequency,
bin N/2 = DC, bin N−1 = highest frequency.

**dBFS normalization:** divide magnitude by `fft_size` before `log10`.
This makes a full-scale sine wave read near 0 dBFS regardless of FFT size.
Use a floor of `−160.0` for zero-magnitude bins (avoid `log10(0) = −∞`).

**FftWorker threading:** FFT is CPU-bound. Use `std::thread::spawn`, not
`tokio::spawn`. Tokio's executor is for I/O; blocking it with CPU work causes
async starvation.

**Lock discipline in rx_callback:** Release the `SdrMetrics` mutex BEFORE
allocating the `Vec<u8>` for the sample channel. Holding the lock during
allocation extends the critical section and risks priority inversion.

**Live FFT size / window changes:** deferred. FftWorker is initialized once
(2048-point Hann). Key bindings `n`/`w` belong to Phase 8b after the worker
supports restart.

---

## Dependency order

```
Cargo.toml + src/main.rs      new crates + module declarations
    ↓
src/state.rs                  FftFrame + last_fft_frame on SdrMetrics
    ↓
src/dsp.rs                    WindowFn + compute_window() with tests
    ↓
src/hardware/device.rs        RxContext definition
src/hardware/mod.rs           pub use RxContext
    ↓
src/fft.rs                    FftWorker with tests
    ↓
src/app.rs + device.rs        wire channel, RxContext, FftWorker, update callback
```

---

## Step 1 — Dependencies + `FftFrame`

**Files:** `Cargo.toml`, `src/main.rs`, `src/state.rs`, `src/app.rs`

- [ ] **Add to `Cargo.toml` `[dependencies]`:**

```toml
crossbeam-channel = "0.5"
rustfft = "6"
num-complex = "0.4"
```

- [ ] **Run `cargo build`** to fetch crates. Expected: `Finished`.

- [ ] **Add to `src/main.rs`** (alongside existing `mod` declarations):

```rust
mod dsp;
mod fft;
```

- [ ] **Add `FftFrame` to `src/state.rs`** before the `SdrMetrics` struct:

```rust
#[derive(Clone)]
pub struct FftFrame {
    /// fftshifted, EMA-smoothed magnitude spectrum in dBFS
    pub bins_dbfs: Vec<f32>,
    /// decaying peak hold, same length as bins_dbfs
    pub peak_hold: Vec<f32>,
    /// mean dBFS of the bottom 10% of bins (noise estimate)
    pub noise_floor: f32,
    pub center_freq_hz: u64,
    pub sample_rate: f64,
    pub timestamp: std::time::Instant,
}
```

- [ ] **Add `last_fft_frame` field to `SdrMetrics`** (after `process_rss_mb`):

```rust
pub last_fft_frame: Option<FftFrame>,
```

- [ ] **Initialize in `App::new()`** inside the `SdrMetrics { ... }` literal:

```rust
last_fft_frame: None,
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 2 — DSP module (`src/dsp.rs`)

**Files:** Create `src/dsp.rs`

- [ ] **Write `src/dsp.rs`:**

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowFn {
    Hann,
    Hamming,
    Blackman,
}

pub fn compute_window(fn_type: WindowFn, size: usize) -> Vec<f32> {
    use std::f64::consts::PI;
    let n = size as f64;
    (0..size)
        .map(|i| {
            let x = 2.0 * PI * i as f64 / (n - 1.0);
            match fn_type {
                WindowFn::Hann     => (0.5 * (1.0 - x.cos())) as f32,
                WindowFn::Hamming  => (0.54 - 0.46 * x.cos()) as f32,
                WindowFn::Blackman => (0.42 - 0.5 * x.cos() + 0.08 * (2.0 * x).cos()) as f32,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hann_endpoints_are_zero() {
        let w = compute_window(WindowFn::Hann, 1024);
        assert!(w[0].abs() < 1e-6, "first = {}", w[0]);
        assert!(w[1023].abs() < 1e-6, "last = {}", w[1023]);
    }

    #[test]
    fn hann_peak_near_center() {
        let w = compute_window(WindowFn::Hann, 1024);
        let peak_idx = w.iter().enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert!((peak_idx as i64 - 511).abs() <= 2, "peak at {}", peak_idx);
    }

    #[test]
    fn hamming_endpoints_nonzero() {
        let w = compute_window(WindowFn::Hamming, 1024);
        assert!(w[0] > 0.05, "Hamming endpoint should not reach zero, got {}", w[0]);
    }

    #[test]
    fn blackman_endpoints_near_zero() {
        let w = compute_window(WindowFn::Blackman, 1024);
        assert!(w[0].abs() < 1e-4, "first = {}", w[0]);
        assert!(w[1023].abs() < 1e-4, "last = {}", w[1023]);
    }
}
```

- [ ] **Run `cargo test dsp::tests`**. Expected: 4 tests pass.

---

## Step 3 — `RxContext` definition

**Files:** `src/hardware/device.rs`, `src/hardware/mod.rs`

`RxContext` bundles the two things `rx_callback` needs: the metrics mutex and
the sample channel sender. Defining it now (without yet changing the callback)
keeps the code compilable between steps.

- [ ] **Add to `src/hardware/device.rs`** before `pub struct Device`:

```rust
pub struct RxContext {
    pub metrics: Arc<Mutex<SdrMetrics>>,
    pub sample_tx: crossbeam_channel::Sender<Vec<u8>>,
}
```

Add the import at the top of the file alongside the existing ones:
```rust
// already present: use std::sync::Mutex;
// add:
use std::sync::Arc;
```

(If `Arc` is not already imported — check the existing imports first.)

- [ ] **Export from `src/hardware/mod.rs`:**

```rust
pub use device::{rx_callback, Device, RxContext};
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 4 — `FftWorker` (`src/fft.rs`)

**Files:** Create `src/fft.rs`

- [ ] **Write `src/fft.rs`:**

```rust
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
        // Before shift: DC at 0. After shift of 8-element array: DC moves to index 4.
        let n = 8usize;
        let mags: Vec<f32> = (0..n).map(|i| i as f32).collect(); // [0,1,2,3,4,5,6,7]
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
        // 0x7F = i8::MAX = 127. Normalized: 127/128 ≈ +0.992
        let byte: u8 = 0x7F;
        let f = byte as i8 as f32 / 128.0;
        assert!((f - 0.9921875).abs() < 1e-6, "got {}", f);
    }

    #[test]
    fn iq_byte_i8_min_converts_correctly() {
        // 0x80 = i8::MIN = -128. Normalized: -128/128 = -1.0
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
```

- [ ] **Run `cargo test fft::tests`**. Expected: 5 tests pass.

---

## Step 5 — Wire App: channel, RxContext, FftWorker, updated callback

**Files:** `src/app.rs`, `src/hardware/device.rs`

This step changes two things simultaneously so the code is never in an
inconsistent state:
1. `rx_callback` in `device.rs` is updated to read from `*const RxContext`
2. `app.rs` is updated to build the `RxContext` and pass its pointer to `start_rx`

- [ ] **Update `rx_callback` in `src/hardware/device.rs`:**

Replace the entire `rx_callback` function body:

```rust
pub extern "C" fn rx_callback(transfer: *mut hackrf_transfer) -> c_int {
    unsafe {
        let t = &*transfer;
        let ctx_ptr = t.rx_ctx as *const RxContext;
        if ctx_ptr.is_null() { return 0; }
        let ctx = &*ctx_ptr;

        let buf = std::slice::from_raw_parts(
            t.buffer as *const u8,
            t.valid_length as usize,
        );

        // Health accumulation — lock held briefly, no allocation inside
        {
            let Ok(mut m) = ctx.metrics.lock() else { return 0; };

            m.bytes_since_last_poll += t.valid_length as u64;

            if t.valid_length < t.buffer_length {
                let dropped_pairs = ((t.buffer_length - t.valid_length) / 2) as u64;
                m.acc_drops += dropped_pairs;
                m.total_drops_session += dropped_pairs;
            }

            let mut saturated: u64 = 0;
            let mut i_sum: i64 = 0;
            let mut q_sum: i64 = 0;
            let mut i_sq: i64 = 0;
            let mut q_sq: i64 = 0;

            for chunk in buf.chunks_exact(2) {
                let i = chunk[0] as i8 as i64;
                let q = chunk[1] as i8 as i64;
                i_sum += i;
                q_sum += q;
                i_sq  += i * i;
                q_sq  += q * q;
                if chunk[0] == 0x80 || chunk[0] == 0x7F { saturated += 1; }
                if chunk[1] == 0x80 || chunk[1] == 0x7F { saturated += 1; }
            }

            let pairs = (buf.len() / 2) as u64;
            m.acc_saturated    += saturated;
            m.acc_i_sum        += i_sum;
            m.acc_q_sum        += q_sum;
            m.acc_i_sq_sum     += i_sq;
            m.acc_q_sq_sum     += q_sq;
            m.acc_sample_count += pairs;

            let now_us = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_micros() as u64)
                .unwrap_or(0);
            if let Some(last_us) = m.acc_last_callback_us {
                let gap = now_us.saturating_sub(last_us);
                m.acc_jitter_sum_us += gap;
                m.acc_jitter_count  += 1;
            }
            m.acc_last_callback_us = Some(now_us);
        }
        // Lock released — allocate outside the critical section
        ctx.sample_tx.try_send(buf.to_vec()).ok();
    }
    0
}
```

- [ ] **Update `src/app.rs`:**

Add imports (replace or extend existing `use crate::hardware` line):

```rust
use crate::fft::FftWorker;
use crate::hardware::{self, RxContext};
```

Add `rx_ctx` field to `App` struct:

```rust
pub struct App {
    state: Arc<Mutex<SdrMetrics>>,
    #[allow(dead_code)]
    device: Arc<hardware::Device>,
    #[allow(dead_code)]
    rx_ctx: Arc<RxContext>,
    events: EventStream,
    show_help: bool,
    engine: ui::LayoutEngine,
}
```

In `App::new()`, after `state` is created and before the polling task spawn, add:

```rust
let (sample_tx, sample_rx) = crossbeam_channel::bounded::<Vec<u8>>(4);

let rx_ctx = Arc::new(RxContext {
    metrics: Arc::clone(&state),
    sample_tx,
});

// FftWorker runs on a real OS thread — it's CPU-bound blocking work
let fft_state = Arc::clone(&state);
std::thread::spawn(move || {
    FftWorker::new(sample_rx, fft_state).run();
});
```

Add `crossbeam_channel` to the use list at the top of `app.rs`:
```rust
// no explicit import needed — used via full path crossbeam_channel::bounded
```

Or add: `use crossbeam_channel;` — either works since the crate is in Cargo.toml.

In the existing polling task spawn, clone `rx_ctx` for the background task:

```rust
let rx_ctx_bg = Arc::clone(&rx_ctx);
// (existing) let state_bg = Arc::clone(&state);
// (existing) let device_bg = Arc::clone(&device);
tokio::spawn(async move {
    // ...
    // Change this line:
    //   let user_param = Arc::as_ptr(&state_bg) as *mut libc::c_void;
    // To:
    let user_param = Arc::as_ptr(&rx_ctx_bg) as *mut libc::c_void;
    // ...
})
```

`state_bg` is still needed for health metric computation inside the polling task — keep it.

Add `rx_ctx` to the `Ok(Self { ... })` return:

```rust
Ok(Self {
    state,
    device,
    rx_ctx,
    events: EventStream::new(Duration::from_millis(100)),
    show_help: false,
    engine,
})
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Run `cargo test`**. Expected: all tests pass including the new dsp/fft/buffer ones.

---

## Step 6 — Final validation

```bash
cargo build --release
cargo test
cargo clippy -- -D warnings
```

Expected: zero errors, zero warnings, all tests pass.

**Smoke test with HackRF connected:**
- Start app, press Space to begin RX
- After ~2 s the `last_fft_frame` should be non-None; add a temporary
  `m.push_log(format!("FFT bins: {}", frame.bins_dbfs.len()))` inside the
  FftWorker write block to confirm, then remove it before Phase 8b
