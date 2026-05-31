/// Background tokio tasks spawned during App initialisation.
///
/// Each function takes the data it needs by value (mostly Arc clones) and
/// returns immediately after calling `tokio::spawn`.  The caller never needs
/// to join — these tasks run for the entire lifetime of the process.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::hardware::{self, Device, RxContext};
use crate::state::{SdrMetrics, THROUGHPUT_HISTORY_LEN};

// ── RX polling task ───────────────────────────────────────────────────────────

/// Polls the HackRF device every 200 ms:
///   - starts / stops RX in response to `state.rx_enabled`
///   - computes throughput, drop rate, ADC saturation, IQ metrics, jitter
///   - writes results back to `state`
pub fn spawn_rx_task(
    state: Arc<Mutex<SdrMetrics>>,
    device: Arc<Device>,
    rx_ctx: Arc<RxContext>,
) {
    tokio::spawn(async move {
        let mut hw_rx_active = false;

        loop {
            let now = Instant::now();

            if hw_rx_active && !device.is_streaming() {
                let _ = device.stop_rx();
                hw_rx_active = false;
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                m.rx_enabled = false;
                m.hw_streaming = false;
                m.push_log("WARNING: Streaming stopped unexpectedly — press [Space] to restart");
            }

            {
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                let elapsed_ms = now.duration_since(m.last_poll_time).as_millis() as u64;
                let bytes = m.bytes_since_last_poll;
                m.bytes_since_last_poll = 0;
                m.last_poll_time = now;

                m.hw_streaming = device.is_streaming();

                if let Some(bps) = (bytes * 1000).checked_div(elapsed_ms) {
                    m.current_throughput_bps = bps;
                    m.actual_sample_rate = (m.current_throughput_bps / 2) as u32;
                    let throughput_kb = m.current_throughput_bps / 1024;
                    if m.throughput_history.len() >= THROUGHPUT_HISTORY_LEN {
                        m.throughput_history.pop_front();
                    }
                    m.throughput_history.push_back(throughput_kb);
                    let actual_sr = m.actual_sample_rate as u64;
                    if m.sample_rate_history.len() >= THROUGHPUT_HISTORY_LEN {
                        m.sample_rate_history.pop_front();
                    }
                    m.sample_rate_history.push_back(actual_sr);
                }
                if let Some(dps) = (m.acc_drops * 1000).checked_div(elapsed_ms) {
                    m.drops_per_sec = dps;
                }
                let drops_snapshot = m.drops_per_sec;
                if m.drop_history.len() >= THROUGHPUT_HISTORY_LEN { m.drop_history.pop_front(); }
                m.drop_history.push_back(drops_snapshot);

                let acc_drops      = m.acc_drops;
                let acc_saturated  = m.acc_saturated;
                let acc_i_sum      = m.acc_i_sum;
                let acc_q_sum      = m.acc_q_sum;
                let acc_i_sq_sum   = m.acc_i_sq_sum;
                let acc_q_sq_sum   = m.acc_q_sq_sum;
                let acc_samples    = m.acc_sample_count;
                let acc_jitter_sum = m.acc_jitter_sum_us;
                let acc_jitter_cnt = m.acc_jitter_count;
                m.acc_drops         = 0;
                m.acc_saturated     = 0;
                m.acc_i_sum         = 0;
                m.acc_q_sum         = 0;
                m.acc_i_sq_sum      = 0;
                m.acc_q_sq_sum      = 0;
                m.acc_sample_count  = 0;
                m.acc_jitter_sum_us = 0;
                m.acc_jitter_count  = 0;

                m.iq_amplitude_hist = m.acc_iq_hist;
                m.acc_iq_hist = [0u64; 32];

                let saturable = acc_samples * 2;
                m.adc_saturation_pct = if saturable > 0 {
                    (acc_saturated as f32 / saturable as f32) * 100.0
                } else {
                    0.0
                };
                if m.adc_saturation_pct > m.adc_saturation_peak {
                    m.adc_saturation_peak = m.adc_saturation_pct;
                }
                let sat_snapshot = m.adc_saturation_pct;
                if m.saturation_history.len() >= THROUGHPUT_HISTORY_LEN { m.saturation_history.pop_front(); }
                m.saturation_history.push_back(sat_snapshot);

                if acc_samples > 0 {
                    let n = acc_samples as f64;
                    m.dc_offset_i = (acc_i_sum as f64 / n / 128.0) as f32;
                    m.dc_offset_q = (acc_q_sum as f64 / n / 128.0) as f32;
                    let i_rms = (acc_i_sq_sum as f64 / n).sqrt();
                    let q_rms = (acc_q_sq_sum as f64 / n).sqrt();
                    if q_rms > 0.0 {
                        m.iq_imbalance_db = (20.0 * (i_rms / q_rms).log10()) as f32;
                    }
                }

                if let Some(jitter) = acc_jitter_sum.checked_div(acc_jitter_cnt) {
                    m.callback_jitter_us = jitter;
                }

                let _ = acc_drops;
            }

            let rx_enabled = state.lock().unwrap_or_else(|e| e.into_inner()).rx_enabled;
            if rx_enabled && !hw_rx_active {
                let user_param = Arc::as_ptr(&rx_ctx) as *mut libc::c_void;
                match device.start_rx(hardware::rx_callback, user_param) {
                    Ok(()) => {
                        hw_rx_active = true;
                        state.lock().unwrap_or_else(|e| e.into_inner()).push_log("RX streaming started");
                    }
                    Err(e) => {
                        let msg = format!("Error starting RX: {}", e);
                        let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                        m.rx_enabled = false;
                        m.push_log(msg);
                    }
                }
            } else if !rx_enabled && hw_rx_active {
                let result = device.stop_rx();
                hw_rx_active = false;
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());
                match result {
                    Ok(()) => m.push_log("RX streaming stopped"),
                    Err(e) => m.push_log(format!("Error stopping RX: {}", e)),
                }
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    });
}

// ── Observer polling task ─────────────────────────────────────────────────────

/// Polls sysfs/proc every 1 s to track which process owns the HackRF device
/// (observer mode only).  Writes device identity and owner info to `state`.
pub fn spawn_observer_task(state: Arc<Mutex<SdrMetrics>>, bus: u32, dev: u32) {
    tokio::spawn(async move {
        let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
        let mut last_owner_cpu: Option<(u64, Instant)> = None;

        loop {
            if let Some(info) = hardware::sysfs::find_hackrf() {
                let owner = hardware::sysfs::find_owner(info.bus, info.dev);
                let mut m = state.lock().unwrap_or_else(|e| e.into_inner());

                m.observer_device    = Some(format!("{} · {}", info.product, info.manufacturer));
                m.observer_serial    = Some(info.serial);
                m.observer_usb       = Some(format!(
                    "High Speed ({} Mbit/s) · {} · Bus {}, Dev {}",
                    info.speed_mbits, info.max_power, info.bus, info.dev
                ));
                m.observer_connected = info.connected_secs.map(fmt_duration);

                if let Some(o) = owner {
                    let cpu_pct = if let Some((last_ticks, last_time)) = last_owner_cpu {
                        let elapsed = last_time.elapsed().as_secs_f64();
                        let delta = o.cpu_ticks.saturating_sub(last_ticks) as f64;
                        if elapsed > 0.0 && ticks_per_sec > 0.0 {
                            (delta / ticks_per_sec / elapsed * 100.0).min(100.0) as f32
                        } else { 0.0 }
                    } else { 0.0 };
                    last_owner_cpu = Some((o.cpu_ticks, Instant::now()));

                    m.observer_owner        = Some(format!("{} (PID {})", o.name, o.pid));
                    m.observer_cmdline      = Some(o.cmdline);
                    m.observer_owner_cpu_pct = cpu_pct;
                    m.observer_owner_ram_mb = o.rss_mb;
                    m.observer_owner_uptime = Some(fmt_duration(o.running_secs));
                } else {
                    last_owner_cpu = None;
                    m.observer_owner        = None;
                    m.observer_cmdline      = None;
                    m.observer_owner_cpu_pct = 0.0;
                    m.observer_owner_ram_mb = 0;
                    m.observer_owner_uptime = None;
                }
            }
            // bus/dev retained to avoid unused-variable warnings; may be used for
            // direct sysfs node lookup in a future improvement.
            let _ = (bus, dev);
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}

// ── Self process resource monitor ────────────────────────────────────────────

/// Measures the app's own CPU usage and RAM every 1 s, writes to `state`.
pub fn spawn_sys_resource_task(state: Arc<Mutex<SdrMetrics>>) {
    tokio::spawn(async move {
        let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
        let mut last_ticks = read_self_stats().map(|(t, _)| t).unwrap_or(0);
        let mut last_time = Instant::now();

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if let Some((total_ticks, rss_mb)) = read_self_stats() {
                let elapsed = last_time.elapsed().as_secs_f64();
                let tick_delta = total_ticks.saturating_sub(last_ticks) as f64;
                let cpu_pct = if elapsed > 0.0 && ticks_per_sec > 0.0 {
                    (tick_delta / ticks_per_sec / elapsed * 100.0).min(100.0) as f32
                } else {
                    0.0
                };
                last_ticks = total_ticks;
                last_time = Instant::now();
                if let Ok(mut m) = state.lock() {
                    m.process_cpu_pct = cpu_pct;
                    m.process_rss_mb  = rss_mb;
                }
            }
        }
    });
}

/// Reads CPU ticks (utime + stime) and RSS in MB from `/proc/self`.
/// Returns `None` if the files are unreadable or unparseable.
pub fn read_self_stats() -> Option<(u64, u64)> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let after_comm = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let rss_kb: u64 = status
        .lines()
        .find(|l| l.starts_with("VmRSS:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()?;
    Some((utime + stime, rss_kb / 1024))
}

// ── Shared utilities ──────────────────────────────────────────────────────────

pub fn fmt_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 { format!("{}h {}m {}s", h, m, s) }
    else if m > 0 { format!("{}m {}s", m, s) }
    else { format!("{}s", s) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proc_stat_field_indices() {
        let fake = "1234 (my process) S 1 1 1 0 -1 4194304 0 0 0 0 42 7 0 0 20 0 1 0 0 0 0";
        let after_comm = fake.rsplit_once(')').unwrap().1;
        let fields: Vec<&str> = after_comm.split_whitespace().collect();
        assert_eq!(fields.get(11), Some(&"42"), "utime at index 11");
        assert_eq!(fields.get(12), Some(&"7"),  "stime at index 12");
    }

    #[test]
    fn fmt_duration_formats_correctly() {
        assert_eq!(fmt_duration(0),    "0s");
        assert_eq!(fmt_duration(45),   "45s");
        assert_eq!(fmt_duration(90),   "1m 30s");
        assert_eq!(fmt_duration(3661), "1h 1m 1s");
    }
}
