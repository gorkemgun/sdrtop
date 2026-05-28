# Phase 11 — HackRF Deep Diagnostics — Log

← [Roadmap](../Roadmap.md) | [Steps](Phase%2011%20-%20HackRF%20Deep%20Diagnostics%20-%20Steps.md)

## Status: ✅ Done

---

## Deviations from plan

### `hackrf_cpld_checksum` not in installed libhackrf

The steps file included a CPLD integrity check via `hackrf_cpld_checksum`. At link time the symbol was missing from the installed `libhackrf.so`. Confirmed with `nm -D`:

```
nm -D /usr/lib/libhackrf.so | grep -i cpld
# → no output
```

**Decision:** Removed CPLD FFI declaration and Device method. `cpld_ok: Option<bool>` kept as `None` permanently in `SdrMetrics`. The RF Chain panel shows `n/a` for CPLD — honest and non-misleading.

### BB filter BW computed, not queried

The plan assumed `hackrf_baseband_filter_bandwidth_set` mirrors a queryable state. libhackrf has no getter for current filter BW. **Decision:** Compute it in Rust using the 16 hardcoded MAX2837 hardware BW steps, taking the step nearest to sample_rate. Implemented as `compute_bb_filter_bw(sample_rate_hz: f64) -> u32` in `device.rs`.

### `ffi::` prefix caused compile error

`device.rs` uses `use super::ffi::*` (glob import), so FFI functions must be called without the `ffi::` prefix. Fixed after first compile attempt.

### `sample_rate` lock ordering in `fft.rs`

The lock that reads `(center_freq_hz, sample_rate)` was originally placed after the SNR/OBW computation that needs `sample_rate`. Fixed by hoisting the lock acquisition above all metric computation.

### `FftFrame` literal needed updating

After adding `snr_db`, `channel_power_dbfs`, `occupied_bw_hz` fields to `FftFrame`, the existing struct literal in `fft.rs` didn't compile. Updated literal with placeholder values before the computation steps filled them properly.

### `bin_color` function unused

The `iq_histogram.rs` originally had a `bin_color(bin: usize)` helper that was superseded by the 3-strip column layout approach. Removed to silence dead_code warning.

### Clippy: needless range loop in `iq_histogram.rs`

Two `for i in 0..n` loops indexed collections by position. Refactored to iterator patterns (`hist.iter().take(n_bins)`, `rows.iter_mut().enumerate().take(bar_height)`).

---

## Key decisions

**IQ histogram uses Chebyshev distance** (`max(|i|, |q|)` on i8) — branchless, no sqrt, appropriate for a hot rx_callback path. 32 bins of width 4 covering amplitude range 0–127.

**Accumulator snapshot pattern** — rx_callback writes to `acc_iq_hist: [u64; 32]` without any UI locking; polling task atomically snapshots to `iq_amplitude_hist` and resets the accumulator. Keeps rx_callback lock-free.

**Log-scale histogram rendering** — linear scale would compress high-amplitude bins into invisibility. Log scale makes bin occupancy visible across orders of magnitude, matching how engineers intuitively read RF amplitude distributions.

**SNR = peak − noise floor, clamped ≥ 0** — simple, fast, and correct for the use case. No SINAD or MER computation (those require demodulation context).

**Channel power = 10·log10(Σ 10^(bin/10))** — standard dBFS power summation across all FFT bins. More meaningful than peak alone for wideband signals.

**99% occupied BW** — sort bins by power descending, accumulate until ≥ 99% of total linear power, then compute frequency span of the selected bins.

**`lab` preset layout** — RF Chain + IQ Diagnostics on Left, Signal Metrics + IQ Histogram + Hardware Health + System Resources on Right. Gives a complete lab-grade view in one keypress (`[6]`).

---

## Files changed

| File | Change |
|---|---|
| `src/hardware/ffi.rs` | Added `hackrf_board_rev_read`, `hackrf_usb_api_version_read` |
| `src/hardware/device.rs` | Added `board_rev()`, `usb_api_version()`, `board_rev_name()`, `compute_bb_filter_bw()` + 6 unit tests |
| `src/state.rs` | Added 7 new `SdrMetrics` fields + 3 `FftFrame` fields |
| `src/fft.rs` | SNR, channel power, occupied BW computation; updated `FftFrame` literal; 3 new tests |
| `src/app.rs` | Reads board_rev/usb_api_version at startup; registers 3 new panels; IQ hist snapshot in poll task; `'6'` key → lab preset |
| `src/config.rs` | Added `lab` preset to `LayoutConfig::default_config()` |
| `src/ui/mod.rs` | Added `rf_chain`, `signal_metrics`, `iq_histogram` modules and re-exports |
| `src/ui/rf_chain.rs` | New — RfChainPanel |
| `src/ui/signal_metrics.rs` | New — SignalMetricsPanel |
| `src/ui/iq_histogram.rs` | New — IqHistogramPanel |
| `src/ui/overlay.rs` | Added `[6] Preset: lab` line; height 21 → 22 |

---

## Test results

```
running 40 tests
... all pass
cargo clippy -- -D warnings  →  Finished (no errors)
```
