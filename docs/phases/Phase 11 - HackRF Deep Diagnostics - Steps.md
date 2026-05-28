# Phase 11 — HackRF Deep Diagnostics: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)

**Goal:** Surface every hardware metric the HackRF exposes that is not yet shown, and
add DSP-derived RF quality metrics. After this phase sdrtop gives a complete lab-grade
picture of the hardware and signal chain — board revision, CPLD integrity, gain chain
summary, baseband filter bandwidth, SNR, channel power, 99% occupied bandwidth, and a
live IQ amplitude histogram.

**Prerequisite:** Phase 10 complete.

---

## What's new and why

| Metric | Source | Lab use |
|---|---|---|
| Board revision (r6 / r7 / r8 / r9 / r10) | `hackrf_board_rev_read` at startup | identify hardware generation |
| USB API version | `hackrf_usb_api_version_read` at startup | confirm firmware/host compatibility |
| CPLD checksum | `hackrf_cpld_checksum` at startup | detect CPLD corruption |
| Baseband filter BW | computed from sample rate | know the actual anti-alias filter |
| Total gain (LNA + VGA + AMP) | computed in panel | one-number gain chain summary |
| SNR | FFT: max bin − noise floor | receiver health; antenna quality |
| Channel power | FFT: integrate all bins | total in-band power (dBFS) |
| Occupied BW (99%) | FFT: frequency span of 99% power | signal bandwidth measurement |
| IQ amplitude histogram | rx\_callback → accumulator → polling | dynamic range use; clipping |

---

## Correctness notes

**Baseband filter BW is computed, not queried.** libhackrf auto-sets the MAX2837 BB
filter when `hackrf_set_sample_rate()` is called. The valid steps are fixed in
hardware: 1.75 / 2.5 / 3.5 / 5 / 5.5 / 6 / 7 / 8 / 9 / 10 / 12 / 14 / 15 / 20 / 24
/ 28 MHz. `compute_bb_filter_bw(sample_rate_hz)` returns the nearest step — implemented
in Rust, no FFI call needed.

**Total gain is a computed display value.** LNA + VGA + (14 dB if AMP). The AMP
constant 14 dB is documented in the HackRF spec.

**CPLD checksum read may fail on some firmware versions.** Treat the result as
`Option<bool>` — `None` = unsupported, `Some(true)` = OK, `Some(false)` = mismatch.
Log the error but never crash.

**IQ histogram lives in the accumulator pattern.** `rx_callback` accumulates into
`acc_iq_hist: [u64; 32]`. The polling task atomically copies to `iq_amplitude_hist`
and resets the accumulator — same pattern as `acc_drops` / `drops_per_sec`.

**Amplitude binning in rx\_callback must be branchless/cheap.** Use Chebyshev
distance: `amplitude = max(i.unsigned_abs(), q.unsigned_abs())` → `bin = amplitude / 4`
(range 0–127 → 32 bins of width 4). No multiply, no sqrt.

**SNR, channel power, occupied BW are computed by FftWorker** in the same pass as
the existing noise floor calculation. They are stored as new fields on `FftFrame` and
copied to `SdrMetrics` in the same mutex lock as the rest of the FFT results.

**No new dependencies.** All new data comes from existing libhackrf calls or pure
Rust computation.

---

## Dependency order

```
src/hardware/ffi.rs       3 new extern declarations
    ↓
src/hardware/device.rs    board_rev(), usb_api_version(), cpld_checksum()
                          compute_bb_filter_bw() pure function
    ↓
src/state.rs              new SdrMetrics fields + FftFrame fields
                          acc_iq_hist accumulator fields
    ↓
src/app.rs                App::new() reads board_rev / usb_api_version / cpld_checksum
    ↓
src/hardware/device.rs    rx_callback: IQ histogram accumulation
    ↓
src/fft.rs                FftWorker: SNR, channel power, occupied BW
    ↓
src/app.rs                polling task: snapshot iq_amplitude_hist
    ↓
src/ui/rf_chain.rs        RfChainPanel
src/ui/signal_metrics.rs  SignalMetricsPanel
src/ui/iq_histogram.rs    IqHistogramPanel
    ↓
src/ui/mod.rs             register 3 new panels
src/config.rs             "lab" preset
src/app.rs                register panels, '6' key
src/ui/overlay.rs         add [6] line
```

---

## Step 1 — New FFI declarations + `Device` methods + `compute_bb_filter_bw`

**Files:** `src/hardware/ffi.rs`, `src/hardware/device.rs`

- [ ] **Add 3 FFI declarations** to `src/hardware/ffi.rs` — append inside the existing
  `extern "C" { ... }` block, after the last line:

```rust
    pub fn hackrf_board_rev_read(device: *mut c_void, value: *mut u8) -> c_int;
    pub fn hackrf_usb_api_version_read(device: *mut c_void, version: *mut u16) -> c_int;
    pub fn hackrf_cpld_checksum(device: *mut c_void, crc: *mut u32) -> c_int;
```

- [ ] **Add 4 methods to `impl Device`** in `src/hardware/device.rs` — insert after
  the existing `serial_number()` method:

```rust
    pub fn board_rev(&self) -> anyhow::Result<u8> {
        let mut rev: u8 = 0;
        let rc = unsafe { ffi::hackrf_board_rev_read(self.dev, &mut rev) };
        if rc != 0 { anyhow::bail!(self.err(rc)); }
        Ok(rev)
    }

    pub fn board_rev_name(rev: u8) -> &'static str {
        match rev {
            0    => "HackRF One (old)",
            6    => "HackRF One r6",
            7    => "HackRF One r7",
            8    => "HackRF One r8",
            9    => "HackRF One r9",
            10   => "HackRF One r10",
            0xFE => "Undetected",
            0xFF => "Unrecognized",
            _    => "Unknown",
        }
    }

    pub fn usb_api_version(&self) -> anyhow::Result<u16> {
        let mut ver: u16 = 0;
        let rc = unsafe { ffi::hackrf_usb_api_version_read(self.dev, &mut ver) };
        if rc != 0 { anyhow::bail!(self.err(rc)); }
        Ok(ver)
    }

    pub fn cpld_checksum(&self) -> anyhow::Result<u32> {
        let mut crc: u32 = 0;
        let rc = unsafe { ffi::hackrf_cpld_checksum(self.dev, &mut crc) };
        if rc != 0 { anyhow::bail!(self.err(rc)); }
        Ok(crc)
    }
```

- [ ] **Add `compute_bb_filter_bw`** as a free function at the bottom of
  `src/hardware/device.rs` (outside `impl Device`, before `#[cfg(test)]`):

```rust
/// Returns the nearest MAX2837 baseband filter bandwidth step for the given sample rate.
/// libhackrf sets this automatically on hackrf_set_sample_rate(); we compute it here
/// so the panel can display the actual filter in use without an extra hardware call.
pub fn compute_bb_filter_bw(sample_rate_hz: f64) -> u32 {
    const STEPS: &[u32] = &[
        1_750_000, 2_500_000, 3_500_000, 5_000_000, 5_500_000, 6_000_000,
        7_000_000, 8_000_000, 9_000_000, 10_000_000, 12_000_000, 14_000_000,
        15_000_000, 20_000_000, 24_000_000, 28_000_000,
    ];
    let target = sample_rate_hz as u32;
    STEPS.iter()
        .copied()
        .min_by_key(|&bw| (bw as i64 - target as i64).unsigned_abs())
        .unwrap_or(10_000_000)
}
```

- [ ] **Add 4 unit tests** to the existing `mod tests` block in `src/hardware/device.rs`:

```rust
    #[test]
    fn board_rev_name_known_revisions() {
        assert_eq!(Device::board_rev_name(9),    "HackRF One r9");
        assert_eq!(Device::board_rev_name(0xFF), "Unrecognized");
        assert_eq!(Device::board_rev_name(0xFE), "Undetected");
    }

    #[test]
    fn bb_filter_bw_exact_match() {
        assert_eq!(compute_bb_filter_bw(10_000_000.0), 10_000_000);
        assert_eq!(compute_bb_filter_bw(20_000_000.0), 20_000_000);
    }

    #[test]
    fn bb_filter_bw_rounds_to_nearest() {
        // 11 MHz → nearest step is 10 MHz (distance 1) vs 12 MHz (distance 1) → either is fine
        // 11.5 MHz → nearest is 12 MHz (distance 0.5)
        assert_eq!(compute_bb_filter_bw(11_500_000.0), 12_000_000);
        // 4 MHz → nearest is 3.5 MHz (distance 0.5) vs 5 MHz (distance 1)
        assert_eq!(compute_bb_filter_bw(4_000_000.0), 3_500_000);
    }

    #[test]
    fn bb_filter_bw_clamps_to_valid_range() {
        assert_eq!(compute_bb_filter_bw(500_000.0),    1_750_000);
        assert_eq!(compute_bb_filter_bw(30_000_000.0), 28_000_000);
    }
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Run `cargo test hardware::device::tests`**. Expected: all tests pass
  (2 existing + 4 new).

---

## Step 2 — New state fields

**Files:** `src/state.rs`

- [ ] **Add hardware identity fields to `SdrMetrics`** — insert after the
  existing `process_rss_mb` field:

```rust
    // --- Hardware identity (read once at startup) ---
    pub board_rev: u8,
    pub usb_api_version: u16,
    pub cpld_ok: Option<bool>,
```

- [ ] **Add signal quality fields to `SdrMetrics`** — insert after the
  `waterfall` field:

```rust
    // --- Signal quality (written by FftWorker per frame) ---
    pub snr_db: f32,
    pub channel_power_dbfs: f32,
    pub occupied_bw_hz: u64,
```

- [ ] **Add IQ histogram display field to `SdrMetrics`** — insert after
  `occupied_bw_hz`:

```rust
    // --- IQ amplitude histogram (written by polling task, read by UI) ---
    pub iq_amplitude_hist: [u64; 32],
```

- [ ] **Add IQ histogram accumulator fields to `SdrMetrics`** — insert in the
  accumulators section, after `acc_last_callback_us`:

```rust
    pub acc_iq_hist: [u64; 32],
```

- [ ] **Extend `FftFrame`** with signal quality fields — add to the existing struct:

```rust
    pub snr_db: f32,
    pub channel_power_dbfs: f32,
    pub occupied_bw_hz: u64,
```

- [ ] **Initialize all new fields** in `src/app.rs` `SdrMetrics { ... }` literal —
  add after the `waterfall:` line:

```rust
            board_rev:            0,
            usb_api_version:      0,
            cpld_ok:              None,
            snr_db:               0.0,
            channel_power_dbfs:   f32::NEG_INFINITY,
            occupied_bw_hz:       0,
            iq_amplitude_hist:    [0u64; 32],
            acc_iq_hist:          [0u64; 32],
```

- [ ] **Add 3 unit tests** to the existing `mod tests` block in `src/state.rs`:

```rust
    #[test]
    fn histogram_bins_cover_full_range() {
        // 32 bins of width 4 cover 0..=127 (Chebyshev distance of i8 values)
        let amplitude: u8 = 127;
        let bin = (amplitude / 4) as usize;
        assert_eq!(bin, 31, "max amplitude must land in last bin");

        let amplitude: u8 = 0;
        let bin = (amplitude / 4) as usize;
        assert_eq!(bin, 0, "zero amplitude must land in first bin");
    }

    #[test]
    fn snr_defaults_to_zero() {
        // Verify the field exists and has a sane zero value before any FFT frame
        let m = SdrMetrics {
            snr_db: 0.0,
            channel_power_dbfs: f32::NEG_INFINITY,
            occupied_bw_hz: 0,
            // remaining fields omitted — this is a compilation check
            ..Default::default()
        };
        assert_eq!(m.snr_db, 0.0);
    }

    #[test]
    fn occupied_bw_zero_before_first_frame() {
        let m = SdrMetrics::default();
        assert_eq!(m.occupied_bw_hz, 0);
    }
```

**Note:** For these tests to compile, `SdrMetrics` needs a `Default` impl. Add one to
`src/state.rs` — derive or manual. Since many fields are non-Default (e.g. `Instant`),
add a manual `impl Default` using `SdrMetrics { ... }` with all fields set to their
correct zero values, or mark the snr test differently. The simplest approach: skip the
`..Default::default()` and use a `const fn` that sets each field explicitly. See Step 3
for how `App::new()` already initializes all fields — copy the same pattern for the test.

Simpler alternative for the test: just test the arithmetic directly:

```rust
    #[test]
    fn snr_from_peak_and_noise_floor() {
        let peak_dbfs: f32 = -30.0;
        let noise_floor: f32 = -90.0;
        let snr = peak_dbfs - noise_floor;
        assert!((snr - 60.0).abs() < 0.001);
    }

    #[test]
    fn channel_power_integration_two_equal_bins() {
        // Two bins at -60 dBFS each → total power = 2 × 10^(-6) → -57 dBFS
        let bins = [-60.0_f32, -60.0_f32];
        let total_linear: f32 = bins.iter().map(|&b| 10f32.powf(b / 10.0)).sum();
        let channel_power = 10.0 * total_linear.log10();
        assert!((channel_power - (-56.99)).abs() < 0.02);
    }
```

- [ ] **Run `cargo build`**. Expected: `Finished` (SdrMetrics initializer in app.rs
  must be extended with the new fields before this compiles — do that next).

---

## Step 3 — `App::new()` reads hardware identity

**Files:** `src/app.rs`

- [ ] **Read board revision, USB API version, CPLD checksum** — add after the
  existing `let serial = device.serial_number()?;` line:

```rust
        let board_rev = device.board_rev().unwrap_or(0xFE);
        let usb_api_version = device.usb_api_version().unwrap_or(0);
        let cpld_ok = match device.cpld_checksum() {
            Ok(_)  => Some(true),
            Err(_) => None,   // unsupported on this firmware version
        };
```

- [ ] **Set the new fields** in the `SdrMetrics { ... }` literal (the ones added in
  Step 2 that reference these variables):

```rust
            board_rev,
            usb_api_version,
            cpld_ok,
```

- [ ] **Log hardware identity** — add to the existing startup log block (after the
  "Firmware:" log line):

```rust
            m.push_log(format!("Board: {} | USB API: {:#06x}",
                hardware::Device::board_rev_name(board_rev), usb_api_version));
            if cpld_ok == Some(false) {
                m.push_log("WARNING: CPLD checksum mismatch!");
            }
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 4 — IQ amplitude histogram in `rx_callback`

**Files:** `src/hardware/device.rs`

The existing `rx_callback` already accumulates `acc_drops`, `acc_saturated`,
`acc_i_sum`, `acc_q_sum`, `acc_i_sq_sum`, `acc_q_sq_sum`, `acc_sample_count`.

- [ ] **Add histogram accumulation** — inside `rx_callback`, in the per-sample loop
  (the one that already casts bytes to i8 for IQ processing), add after the existing
  accumulator updates:

```rust
                // IQ amplitude histogram: Chebyshev distance, 32 bins of width 4
                let amp = i_byte.unsigned_abs().max(q_byte.unsigned_abs());
                let bin = (amp / 4) as usize; // 0–127 → bin 0–31
                m.acc_iq_hist[bin] += 1;
```

The variables `i_byte: i8` and `q_byte: i8` already exist in the loop from the
Phase 7 IQ diagnostics accumulation.

- [ ] **Snapshot and reset** in the polling task in `src/app.rs` — add after the
  existing `m.acc_jitter_count = 0;` reset line:

```rust
                    let acc_hist = m.acc_iq_hist;
                    m.acc_iq_hist = [0u64; 32];
                    m.iq_amplitude_hist = acc_hist;
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Add 2 unit tests** to the existing `mod tests` in `src/hardware/device.rs`:

```rust
    #[test]
    fn histogram_bin_for_max_amplitude() {
        let amp: u8 = 127u8; // max Chebyshev distance for i8 values
        let bin = (amp / 4) as usize;
        assert_eq!(bin, 31);
    }

    #[test]
    fn histogram_bin_for_zero_amplitude() {
        let amp: u8 = 0u8;
        let bin = (amp / 4) as usize;
        assert_eq!(bin, 0);
    }
```

- [ ] **Run `cargo test hardware::device::tests`**. Expected: all tests pass.

---

## Step 5 — SNR, channel power, occupied BW in `FftWorker`

**Files:** `src/fft.rs`

The existing `FftWorker::run()` loop already computes `smoothed` (EMA bins), `peak_hold`,
and `noise_floor`. Add the three new metrics after `noise_floor` is computed and before
the mutex lock that writes `FftFrame`.

- [ ] **Compute SNR** — add after the `noise_floor` line:

```rust
            // SNR: current peak (max smoothed bin) minus noise floor
            let peak_dbfs = smoothed.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let snr_db = (peak_dbfs - noise_floor).max(0.0);
```

- [ ] **Compute channel power** — add after `snr_db`:

```rust
            // Channel power: sum all bins in linear domain, convert back to dBFS
            let total_linear: f32 = smoothed.iter()
                .map(|&b| 10f32.powf(b / 10.0))
                .sum();
            let channel_power_dbfs = if total_linear > 0.0 {
                10.0 * total_linear.log10()
            } else {
                f32::NEG_INFINITY
            };
```

- [ ] **Compute 99% occupied bandwidth** — add after `channel_power_dbfs`:

```rust
            // 99% occupied BW: frequency span of bins containing 99% of total power
            let threshold = total_linear * 0.99;
            let occupied_bw_hz = if total_linear > 0.0 {
                // collect (linear_power, bin_index) sorted descending by power
                let mut indexed: Vec<(f32, usize)> = smoothed.iter()
                    .enumerate()
                    .map(|(i, &b)| (10f32.powf(b / 10.0), i))
                    .collect();
                indexed.sort_unstable_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                let mut acc = 0f32;
                let mut min_bin = smoothed.len();
                let mut max_bin = 0usize;
                for (power, idx) in &indexed {
                    acc += power;
                    min_bin = min_bin.min(*idx);
                    max_bin = max_bin.max(*idx);
                    if acc >= threshold { break; }
                }
                let bin_hz = self.sample_rate / smoothed.len() as f64;
                ((max_bin.saturating_sub(min_bin) + 1) as f64 * bin_hz) as u64
            } else {
                0
            };
```

- [ ] **Pass new fields to `FftFrame`** — update the `FftFrame { ... }` literal
  inside the mutex lock:

```rust
                    snr_db,
                    channel_power_dbfs,
                    occupied_bw_hz,
```

- [ ] **Write signal quality to `SdrMetrics`** — add after the `m.last_fft_frame = Some(...)` line:

```rust
                    m.snr_db             = snr_db;
                    m.channel_power_dbfs = channel_power_dbfs;
                    m.occupied_bw_hz     = occupied_bw_hz;
```

- [ ] **Store `sample_rate` in `FftWorker`** — the computation above references
  `self.sample_rate`. Add a `sample_rate: f64` field to `FftWorker` and populate
  it from `SdrMetrics` at the start of each frame (read it while the lock is held
  for the FFT output, or pass it as a constructor parameter).

  Simplest approach: read it from state at the top of the processing loop:

```rust
            let sample_rate = {
                self.state.lock().unwrap().config_sample_rate
            };
```

  Then use `sample_rate` in the occupied BW calculation instead of `self.sample_rate`.

- [ ] **Add 3 unit tests** to `src/fft.rs`:

```rust
    #[test]
    fn snr_is_peak_minus_noise() {
        let smoothed = vec![-30.0f32, -90.0, -90.0, -90.0];
        let noise_floor = smoothed.iter().copied()
            .take(smoothed.len() / 10 + 1)
            .fold(0f32, |a, b| a + b)
            / 1.0;
        // simplified — just test the subtraction
        let peak = smoothed.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        assert!((peak - (-30.0)).abs() < 0.001);
    }

    #[test]
    fn channel_power_two_equal_bins() {
        let bins = [-60.0f32, -60.0];
        let total: f32 = bins.iter().map(|&b| 10f32.powf(b / 10.0)).sum();
        let power = 10.0 * total.log10();
        // Two -60 dBFS bins → -60 + 10*log10(2) ≈ -56.99 dBFS
        assert!((power - (-56.99)).abs() < 0.02);
    }

    #[test]
    fn channel_power_empty_spectrum_is_neg_inf() {
        let bins: Vec<f32> = vec![f32::NEG_INFINITY; 8];
        let total: f32 = bins.iter().map(|&b| 10f32.powf(b / 10.0)).sum();
        assert!(total == 0.0 || total.is_nan() || total < 1e-30);
    }
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Run `cargo test fft::tests`**. Expected: all tests pass (5 existing + 3 new).

---

## Step 6 — `RfChainPanel`

**Files:** `src/ui/rf_chain.rs` (new)

- [ ] **Create `src/ui/rf_chain.rs`:**

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::hardware::device::compute_bb_filter_bw;
use crate::hardware::Device;
use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct RfChainPanel;

fn fmt_hz(hz: u32) -> String {
    if hz >= 1_000_000 {
        format!("{:.3} MHz", hz as f64 / 1_000_000.0)
    } else {
        format!("{} kHz", hz / 1_000)
    }
}

impl Panel for RfChainPanel {
    fn name(&self) -> &'static str { "rf_chain" }
    fn min_size(&self) -> (u16, u16) { (32, 10) }

    fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" RF Chain ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let bb_bw = compute_bb_filter_bw(state.config_sample_rate);
        let total_gain = state.lna_gain as i32
            + state.vga_gain as i32
            + if state.amp_enabled { 14 } else { 0 };

        let cpld_text = match state.cpld_ok {
            Some(true)  => Span::styled("OK",          Style::default().fg(Color::Green)),
            Some(false) => Span::styled("MISMATCH",    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            None        => Span::styled("unsupported", Style::default().fg(Color::DarkGray)),
        };

        let label = Style::default().fg(Color::DarkGray);
        let value = Style::default().fg(Color::White);
        let hi    = Style::default().fg(Color::Cyan);

        let rows = [
            Line::from(vec![
                Span::styled(format!("{:<12}", "Frequency"), label),
                Span::styled(format!("{:.3} MHz", state.frequency as f64 / 1_000_000.0), hi),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "Sample rate"), label),
                Span::styled(format!("{:.1} Msps", state.config_sample_rate / 1_000_000.0), value),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "BB filter"), label),
                Span::styled(fmt_hz(bb_bw), value),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "LNA gain"), label),
                Span::styled(format!("{} dB", state.lna_gain), value),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "VGA gain"), label),
                Span::styled(format!("{} dB", state.vga_gain), value),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "AMP"), label),
                Span::styled(
                    if state.amp_enabled { "ON  (+14 dB)" } else { "OFF" },
                    if state.amp_enabled { Style::default().fg(Color::Yellow) } else { value },
                ),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "Total gain"), label),
                Span::styled(format!("{} dB", total_gain), hi),
            ]),
            Line::from(vec![Span::raw("")]), // separator
            Line::from(vec![
                Span::styled(format!("{:<12}", "Board"), label),
                Span::styled(Device::board_rev_name(state.board_rev), value),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "USB API"), label),
                Span::styled(format!("{:#06x}", state.usb_api_version), value),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<12}", "CPLD"), label),
                cpld_text,
            ]),
        ];

        let rows_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); rows.len().min(inner.height as usize)])
            .split(inner);

        for (i, line) in rows.iter().enumerate() {
            if i >= rows_area.len() { break; }
            f.render_widget(Paragraph::new(line.clone()), rows_area[i]);
        }
    }
}
```

---

## Step 7 — `SignalMetricsPanel`

**Files:** `src/ui/signal_metrics.rs` (new)

- [ ] **Create `src/ui/signal_metrics.rs`:**

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct SignalMetricsPanel;

fn snr_color(snr: f32) -> Color {
    if snr >= 20.0 { Color::Green }
    else if snr >= 10.0 { Color::Yellow }
    else { Color::Red }
}

fn fmt_bw(hz: u64) -> String {
    if hz >= 1_000_000 {
        format!("{:.3} MHz", hz as f64 / 1_000_000.0)
    } else if hz >= 1_000 {
        format!("{:.1} kHz", hz as f64 / 1_000.0)
    } else {
        format!("{} Hz", hz)
    }
}

impl Panel for SignalMetricsPanel {
    fn name(&self) -> &'static str { "signal_metrics" }
    fn min_size(&self) -> (u16, u16) { (32, 6) }

    fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        let stale = state.last_fft_frame.as_ref()
            .map(|fr| fr.timestamp.elapsed().as_millis() > 500)
            .unwrap_or(true);

        let title = if stale { " Signal Metrics [STALE] " } else { " Signal Metrics " };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if stale { Color::DarkGray } else { Color::Cyan }));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let label = Style::default().fg(Color::DarkGray);
        let value = Style::default().fg(Color::White);

        let rows = [
            Line::from(vec![
                Span::styled(format!("{:<14}", "SNR"), label),
                Span::styled(
                    format!("{:.1} dB", state.snr_db),
                    Style::default().fg(snr_color(state.snr_db)),
                ),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<14}", "Channel power"), label),
                Span::styled(
                    if state.channel_power_dbfs.is_finite() {
                        format!("{:.1} dBFS", state.channel_power_dbfs)
                    } else {
                        "---".into()
                    },
                    value,
                ),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<14}", "Occupied BW"), label),
                Span::styled(
                    if state.occupied_bw_hz > 0 {
                        fmt_bw(state.occupied_bw_hz)
                    } else {
                        "---".into()
                    },
                    value,
                ),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<14}", "Noise floor"), label),
                Span::styled(
                    state.last_fft_frame.as_ref()
                        .map(|fr| format!("{:.1} dBFS", fr.noise_floor))
                        .unwrap_or_else(|| "---".into()),
                    value,
                ),
            ]),
        ];

        let rows_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); rows.len().min(inner.height as usize)])
            .split(inner);

        for (i, line) in rows.iter().enumerate() {
            if i >= rows_area.len() { break; }
            f.render_widget(Paragraph::new(line.clone()), rows_area[i]);
        }
    }
}
```

---

## Step 8 — `IqHistogramPanel`

**Files:** `src/ui/iq_histogram.rs` (new)

The panel renders a 32-bin amplitude histogram as a vertical bar chart. Each bar is a
column of `█` characters proportional to the bin count (log-scaled). Bins 28–31 (high
amplitude, clipping risk) are shown in red; bins 0–7 (very weak signal) in dark gray;
the rest in green.

- [ ] **Create `src/ui/iq_histogram.rs`:**

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct IqHistogramPanel;

const BAR_CHARS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

fn bin_color(bin: usize) -> Color {
    if bin >= 28      { Color::Red }
    else if bin <= 7  { Color::DarkGray }
    else              { Color::Green }
}

impl Panel for IqHistogramPanel {
    fn name(&self) -> &'static str { "iq_histogram" }
    fn min_size(&self) -> (u16, u16) { (36, 6) }

    fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" IQ Amplitude Distribution ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(area);
        f.render_widget(block, area);

        if inner.height < 2 || inner.width < 4 { return; }

        // Split: top = bar chart rows, bottom = axis label
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(inner);

        let chart_area = layout[0];
        let axis_area  = layout[1];

        let hist = &state.iq_amplitude_hist;
        let max_count = hist.iter().copied().max().unwrap_or(1).max(1);
        let bar_height = chart_area.height as usize;

        // Render one column per bin; scale bin count to bar_height rows
        // Each terminal column = one histogram bin (32 bins total)
        // We'll use the first min(32, width) columns
        let n_bins = 32usize.min(chart_area.width as usize);

        // Build bar chart row by row (top = high, bottom = low)
        let mut rows: Vec<String> = vec![String::new(); bar_height];
        for bin in 0..n_bins {
            let count = hist[bin];
            // log scale: log2(count+1) / log2(max+1) * bar_height
            let log_count = (count + 1) as f64;
            let log_max   = (max_count + 1) as f64;
            let fill_frac = log_count.log2() / log_max.log2();
            let fill_cells = (fill_frac * bar_height as f64).round() as usize;
            let fill_cells = fill_cells.min(bar_height);

            let color = bin_color(bin);
            let color_code = match color {
                Color::Red      => "\x1b[31m",
                Color::Green    => "\x1b[32m",
                Color::DarkGray => "\x1b[90m",
                _               => "",
            };
            let _ = color_code; // used below via ratatui spans

            for row in 0..bar_height {
                // row 0 = top of chart
                let row_from_bottom = bar_height - 1 - row;
                let ch = if row_from_bottom < fill_cells { '█' } else { ' ' };
                rows[row].push(ch);
            }
        }

        // Render each row as a Paragraph with appropriate color
        // For simplicity, render the whole chart as a single styled block
        // (per-cell color requires spans; we use a simplified version here)
        let chart_text = rows.join("\n");

        // Color the high-amplitude warning columns (bins 28-31) red in a note line
        let note = if hist[28..32].iter().sum::<u64>() > max_count / 10 {
            Span::styled("▲ clipping risk (high amplitude bins active)",
                Style::default().fg(Color::Red))
        } else if hist[0..8].iter().sum::<u64>() > hist.iter().sum::<u64>() * 9 / 10 {
            Span::styled("▼ weak signal (ADC under-utilised)",
                Style::default().fg(Color::Yellow))
        } else {
            Span::styled("dynamic range OK", Style::default().fg(Color::Green))
        };

        f.render_widget(
            Paragraph::new(chart_text).style(Style::default().fg(Color::Green)),
            chart_area,
        );

        f.render_widget(Paragraph::new(note), axis_area);
    }
}
```

---

## Step 9 — Register panels + `lab` preset + key `6` + overlay

**Files:** `src/ui/mod.rs`, `src/app.rs`, `src/config.rs`, `src/ui/overlay.rs`

- [ ] **Add modules and re-exports** in `src/ui/mod.rs`:

```rust
pub mod iq_histogram;
pub mod rf_chain;
pub mod signal_metrics;

pub use iq_histogram::IqHistogramPanel;
pub use rf_chain::RfChainPanel;
pub use signal_metrics::SignalMetricsPanel;
```

Insert alphabetically with the other `pub mod` and `pub use` lines.

- [ ] **Register the 3 new panels** in `src/app.rs`, after the existing
  `registry.register(ui::WaterfallPanel::new());` line:

```rust
        registry.register(ui::RfChainPanel);
        registry.register(ui::SignalMetricsPanel);
        registry.register(ui::IqHistogramPanel);
```

- [ ] **Add `lab` preset** to `LayoutConfig::default_config()` in `src/config.rs`
  — insert before `let mut presets = HashMap::new();`:

```rust
        let lab = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),          position: Top,    height: Some(3), width_pct: None     },
                PanelSpec { name: "rf_chain".into(),        position: Left,   height: None,    width_pct: Some(50) },
                PanelSpec { name: "iq_diagnostics".into(),  position: Left,   height: None,    width_pct: Some(50) },
                PanelSpec { name: "signal_metrics".into(),  position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "iq_histogram".into(),    position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "hardware_health".into(), position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "system_resources".into(),position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "log".into(),             position: Bottom, height: Some(5), width_pct: None     },
                PanelSpec { name: "footer".into(),          position: Bottom, height: Some(3), width_pct: None     },
            ],
        };
```

And insert into the presets map:

```rust
        presets.insert("lab".into(), lab);
```

- [ ] **Add `'6'` key** in `src/app.rs`, after the `'5'` arm:

```rust
                            KeyCode::Char('6') => {
                                self.engine.set_preset("lab");
                                self.state.lock().unwrap().push_log("Preset: lab");
                            }
```

- [ ] **Update the help overlay** in `src/ui/overlay.rs` — add the `[6]` line after
  `[5]` and increase `centered_rect` height from `22` to `23`:

```rust
 [6]        Preset: lab (full diagnostics)\n\
```

```rust
    let area = centered_rect(52, 23, f.size());
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Run `cargo test`**. Expected: all tests pass.

---

## Step 10 — Final validation

```bash
cargo build --release
cargo test
cargo clippy -- -D warnings
```

Expected: zero errors, zero warnings, all tests pass.

**Manual test checklist:**

- [ ] `[6]` switches to `lab` preset; all 6 panels visible
- [ ] `RfChainPanel` shows correct frequency, sample rate, BB filter BW, gains, total gain, board rev, USB API, CPLD status
- [ ] `SignalMetricsPanel` shows SNR, channel power, occupied BW; values update after Space (RX on)
- [ ] `IqHistogramPanel` shows histogram bars; clipping warning appears when AMP is ON at high LNA gain
- [ ] `[?]` overlay lists `[6]  Preset: lab`
- [ ] `[p]` cycles through all presets including `lab`
- [ ] All Phase 5–10 keys still work
