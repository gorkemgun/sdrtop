# Phase 7 — Hardware Health Panels: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)

**Goal:** Expose sample drop rate, ADC saturation, IQ quality, and system resource usage
as live-updating panels. These are the metrics that turn sdrtop from an SDR frontend into
a genuine resource monitor. Three new `Panel` implementations plug directly into the Phase 6
registry — no layout code changes needed.

---

## Correctness notes before starting

**`rx_callback` runs on a libhackrf C thread.** It holds `Arc<Mutex<SdrMetrics>>` for its
entire duration. Two rules apply:

1. **No floating-point arithmetic inside the callback.** Accumulate integer sums only.
   The polling task does the float math every 200 ms — it has time, the callback does not.
2. **No allocation inside the callback.** No `Vec::push`, no `String::from`, no `format!`.

**`/proc/self/stat` field 2 is the process name in parentheses** and can contain spaces
and nested parentheses. Splitting by whitespace blindly gives wrong field indices. The safe
approach: find the *last* `)` in the string with `rsplit_once(')')`, then split the
remainder. Fields after the closing paren: index 11 = utime, index 12 = stime.

---

## Dependency order

```
src/state.rs           new SdrMetrics fields + accumulator fields
    ↓
src/hardware/device.rs  rx_callback: integer accumulation only
    ↓
src/app.rs             polling task: compute derived metrics + reset accumulators
                       system resource task: /proc polling every 1 s
    ↓
src/ui/hardware_health.rs   HardwareHealthPanel
src/ui/iq_diagnostics.rs    IqDiagnosticsPanel   (can be done in parallel with above)
src/ui/system_resources.rs  SystemResourcesPanel
    ↓
src/config.rs + src/app.rs  monitoring preset + panel registration + 2 key
```

---

## Step 1 — New fields in `SdrMetrics` (`src/state.rs`)

Fields are grouped by who writes them. The polling task only reads accumulator fields and
immediately resets them after computing derived values.

Add to the `SdrMetrics` struct:

```rust
// --- Derived metrics (written by polling task, read by UI) ---

pub drops_per_sec: u64,
pub total_drops_session: u64,
pub drop_history: VecDeque<u64>,         // 64-point sparkline, drops/sec

pub adc_saturation_pct: f32,             // current poll cycle, 0.0–100.0
pub adc_saturation_peak: f32,            // session maximum
pub saturation_history: VecDeque<f32>,   // 64-point sparkline

pub iq_imbalance_db: f32,               // positive = I stronger, negative = Q stronger
pub dc_offset_i: f32,                   // normalized −1.0..+1.0
pub dc_offset_q: f32,

pub callback_jitter_us: u64,            // rolling mean of inter-callback interval, µs

pub process_cpu_pct: f32,               // written by system resource task
pub process_rss_mb: u64,

// --- Accumulators (written by rx_callback, reset by polling task) ---

pub acc_drops: u64,                     // drop count since last poll
pub acc_saturated: u64,                 // saturated sample count since last poll
pub acc_i_sum: i64,                     // sum of signed I values since last poll
pub acc_q_sum: i64,                     // sum of signed Q values since last poll
pub acc_i_sq_sum: i64,                  // sum of I² since last poll
pub acc_q_sq_sum: i64,                  // sum of Q² since last poll
pub acc_sample_count: u64,              // total IQ sample pairs since last poll
pub acc_jitter_sum_us: u64,             // sum of inter-callback intervals, µs
pub acc_jitter_count: u64,
pub acc_last_callback_us: Option<u64>,  // timestamp of last callback (µs since epoch)
```

Initialize all new fields to zero / `None` / empty `VecDeque` in `App::new()` alongside
the existing fields.

```bash
cargo build
```

Expected: `Finished` with no errors.

---

## Step 2 — `rx_callback`: integer accumulation only (`src/hardware/device.rs`)

The callback must be **fast**. It accumulates raw counts and sums as integers.
No float arithmetic, no allocation.

HackRF IQ bytes are signed 8-bit, interleaved `[I0, Q0, I1, Q1, …]`.
ADC saturation: a byte at its signed minimum (`0x80u8`, i.e. `−128i8`) or signed maximum
(`0x7Fu8`, i.e. `127i8`) has hit the ADC rail.

Drop detection: `hackrf_transfer.valid_length` is the number of bytes actually received.
`hackrf_transfer.buffer_length` is what was requested. If `valid_length < buffer_length`,
the hardware dropped `(buffer_length − valid_length) / 2` IQ pairs.

Replace the body of `rx_callback` in `src/hardware/device.rs`:

```rust
pub extern "C" fn rx_callback(transfer: *mut hackrf_transfer) -> c_int {
    unsafe {
        let t = &*transfer;
        let metrics_ptr = t.rx_ctx as *const Mutex<SdrMetrics>;
        if metrics_ptr.is_null() { return 0; }
        let Ok(mut m) = (*metrics_ptr).lock() else { return 0; };

        let buf = std::slice::from_raw_parts(
            t.buffer as *const u8,
            t.valid_length as usize,
        );

        // Throughput (existing)
        m.bytes_since_last_poll += t.valid_length as u64;

        // Drop detection
        if t.valid_length < t.buffer_length {
            let dropped_pairs = ((t.buffer_length - t.valid_length) / 2) as u64;
            m.acc_drops += dropped_pairs;
            m.total_drops_session += dropped_pairs;
        }

        // IQ accumulation — integer only, no float
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
            // Saturation: signed 8-bit rails are 0x80 (−128) and 0x7F (+127)
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

        // Jitter: time since last callback, in µs
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
    0
}
```

Add a unit test at the bottom of `src/hardware/device.rs`:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn saturation_byte_detection() {
        // 0x7F = +127 (i8 max), 0x80 = -128 (i8 min) — both are ADC rails
        let at_max: u8 = 0x7F;
        let at_min: u8 = 0x80;
        let normal: u8 = 0x40;
        assert!(at_max == 0x7F || at_max == 0x80);
        assert!(at_min == 0x7F || at_min == 0x80);
        assert!(normal != 0x7F && normal != 0x80);
    }

    #[test]
    fn drop_detection_arithmetic() {
        let buffer_length: i32 = 262144; // typical HackRF transfer size
        let valid_length: i32  = 262144 - 128;
        let dropped_pairs = ((buffer_length - valid_length) / 2) as u64;
        assert_eq!(dropped_pairs, 64);
    }
}
```

```bash
cargo build
cargo test hardware::device::tests
```

Expected: both tests pass.

---

## Step 3 — Polling task: compute derived metrics (`src/app.rs`)

In the existing background `tokio::spawn` polling loop, after the throughput block,
add a new block that reads the accumulators, computes derived values, updates sparklines,
then **resets all accumulators**. The accumulator reset must happen in the same lock
acquisition as the read — no window where the callback could half-fill a reset accumulator.

Add a `last_drop_total: u64` variable before the loop (initialized to 0) to track the
delta between poll cycles.

Inside the loop, after the existing throughput computation, while still holding the lock:

```rust
// Snapshot and reset accumulators atomically
let acc_drops      = m.acc_drops;
let acc_saturated  = m.acc_saturated;
let acc_i_sum      = m.acc_i_sum;
let acc_q_sum      = m.acc_q_sum;
let acc_i_sq_sum   = m.acc_i_sq_sum;
let acc_q_sq_sum   = m.acc_q_sq_sum;
let acc_samples    = m.acc_sample_count;
let acc_jitter_sum = m.acc_jitter_sum_us;
let acc_jitter_cnt = m.acc_jitter_count;
m.acc_drops            = 0;
m.acc_saturated        = 0;
m.acc_i_sum            = 0;
m.acc_q_sum            = 0;
m.acc_i_sq_sum         = 0;
m.acc_q_sq_sum         = 0;
m.acc_sample_count     = 0;
m.acc_jitter_sum_us    = 0;
m.acc_jitter_count     = 0;

// Drop rate (per second, not per poll)
if elapsed_ms > 0 {
    m.drops_per_sec = acc_drops * 1000 / elapsed_ms as u64;
}
if m.drop_history.len() >= THROUGHPUT_HISTORY_LEN { m.drop_history.pop_front(); }
m.drop_history.push_back(m.drops_per_sec);

// ADC saturation %
// acc_samples is IQ pairs; each pair has 2 bytes that can saturate
let saturable = acc_samples * 2;
m.adc_saturation_pct = if saturable > 0 {
    (acc_saturated as f32 / saturable as f32) * 100.0
} else {
    0.0
};
if m.adc_saturation_pct > m.adc_saturation_peak {
    m.adc_saturation_peak = m.adc_saturation_pct;
}
if m.saturation_history.len() >= THROUGHPUT_HISTORY_LEN { m.saturation_history.pop_front(); }
m.saturation_history.push_back(m.adc_saturation_pct);

// IQ diagnostics — float math only here, not in callback
if acc_samples > 0 {
    let n = acc_samples as f64;
    // DC offset: mean value normalized to −1.0..+1.0 (128 = full-scale)
    m.dc_offset_i = (acc_i_sum as f64 / n / 128.0) as f32;
    m.dc_offset_q = (acc_q_sum as f64 / n / 128.0) as f32;
    // IQ imbalance: RMS power ratio in dB
    let i_rms = (acc_i_sq_sum as f64 / n).sqrt();
    let q_rms = (acc_q_sq_sum as f64 / n).sqrt();
    if q_rms > 0.0 {
        m.iq_imbalance_db = (20.0 * (i_rms / q_rms).log10()) as f32;
    }
}

// Callback jitter: rolling mean of inter-callback intervals
if acc_jitter_cnt > 0 {
    m.callback_jitter_us = acc_jitter_sum / acc_jitter_cnt;
}
```

Add unit tests in `src/app.rs`:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn iq_imbalance_zero_for_balanced() {
        let n = 1000_f64;
        let i_rms = (500_000_f64 / n).sqrt(); // i_sq_sum = 500_000
        let q_rms = (500_000_f64 / n).sqrt();
        let imbalance = (20.0 * (i_rms / q_rms).log10()) as f32;
        assert!(imbalance.abs() < 0.001, "got {}", imbalance);
    }

    #[test]
    fn iq_imbalance_positive_when_i_stronger() {
        let n = 1000_f64;
        let i_rms = (800_000_f64 / n).sqrt();
        let q_rms = (200_000_f64 / n).sqrt();
        let imbalance = (20.0 * (i_rms / q_rms).log10()) as f32;
        assert!(imbalance > 0.0, "got {}", imbalance);
    }

    #[test]
    fn adc_saturation_pct_full() {
        let acc_saturated = 200_u64;
        let saturable = 100_u64 * 2; // 100 pairs, 2 bytes each
        let pct = (acc_saturated as f32 / saturable as f32) * 100.0;
        assert!((pct - 100.0).abs() < 0.01, "got {}", pct);
    }
}
```

```bash
cargo build
cargo test app::tests
```

Expected: all three tests pass.

---

## Step 4 — System resource polling task (`src/app.rs`)

Spawn a second tokio task that reads `/proc/self/stat` and `/proc/self/status`
every second. This task is completely independent from the hardware polling task.

**`/proc/self/stat` parsing:** field 2 is the process name wrapped in parentheses
and can contain spaces. `rsplit_once(')')` finds the last `)` safely regardless of the name.
Fields after that closing paren (0-indexed): 11 = utime, 12 = stime.

Add the helper function in `src/app.rs` (outside `impl App`):

```rust
fn read_process_stats() -> Option<(u64, u64)> {
    // CPU ticks: use rsplit_once to skip past process name which may contain spaces/parens
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let after_comm = stat.rsplit_once(')')?.1;
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;

    // RSS in MB
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
```

Add a unit test for the parsing logic:

```rust
#[test]
fn proc_stat_field_indices() {
    // Simulate /proc/self/stat after ')': " S 1 1 1 0 -1 0 0 0 0 0 <utime> <stime> ..."
    //                                      0 1 2 3 4  5 6 7 8 9 10  11       12
    let after_comm = " S 0 0 0 0 0 0 0 0 0 0 42 7 0 0 20 0 1 0 0 0 0 0 0 0 0 0";
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    assert_eq!(fields.get(11), Some(&"42"), "utime field");
    assert_eq!(fields.get(12), Some(&"7"),  "stime field");
}
```

Spawn the task in `App::new()` after the existing polling task:

```rust
let sys_state = Arc::clone(&state);
tokio::spawn(async move {
    let ticks_per_sec = unsafe { libc::sysconf(libc::_SC_CLK_TCK) } as f64;
    let mut last_ticks: u64 = 0;
    let mut last_time = std::time::Instant::now();

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if let Some((total_ticks, rss_mb)) = read_process_stats() {
            let elapsed = last_time.elapsed().as_secs_f64();
            let tick_delta = total_ticks.saturating_sub(last_ticks) as f64;
            let cpu_pct = if elapsed > 0.0 && ticks_per_sec > 0.0 {
                (tick_delta / ticks_per_sec / elapsed * 100.0).min(100.0) as f32
            } else {
                0.0
            };
            last_ticks = total_ticks;
            last_time = std::time::Instant::now();
            if let Ok(mut m) = sys_state.lock() {
                m.process_cpu_pct = cpu_pct;
                m.process_rss_mb  = rss_mb;
            }
        }
    }
});
```

```bash
cargo build
cargo test app::tests
```

Expected: all tests including `proc_stat_field_indices` pass.

---

## Step 5 — `HardwareHealthPanel` (`src/ui/hardware_health.rs`)

Create `src/ui/hardware_health.rs`. The panel shows three sections stacked vertically:
drop rate, ADC saturation, and callback jitter. Color thresholds match the design spec.

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Sparkline},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct HardwareHealthPanel;

fn threshold_color(value: f64, warn: f64, crit: f64) -> Color {
    if value >= crit      { Color::Red    }
    else if value >= warn { Color::Yellow }
    else                  { Color::Green  }
}

impl Panel for HardwareHealthPanel {
    fn name(&self) -> &'static str { "hardware_health" }
    fn min_size(&self) -> (u16, u16) { (30, 12) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" Hardware Health ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // drop rate label
                Constraint::Length(2), // drop sparkline
                Constraint::Length(1), // saturation label
                Constraint::Length(2), // saturation sparkline
                Constraint::Length(1), // jitter label
                Constraint::Min(0),
            ])
            .split(inner);

        // Drop rate
        let drop_color = threshold_color(state.drops_per_sec as f64, 1.0, 10.0);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "Drops: {}/s  (session total: {})",
                    state.drops_per_sec, state.total_drops_session
                ),
                Style::default().fg(drop_color),
            )),
            rows[0],
        );
        let drop_data: Vec<u64> = state.drop_history.iter().cloned().collect();
        f.render_widget(
            Sparkline::default()
                .data(&drop_data)
                .style(Style::default().fg(drop_color)),
            rows[1],
        );

        // ADC saturation
        let sat_color = threshold_color(state.adc_saturation_pct as f64, 1.0, 5.0);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "ADC sat: {:.1}%  (peak: {:.1}%)",
                    state.adc_saturation_pct, state.adc_saturation_peak
                ),
                Style::default().fg(sat_color),
            )),
            rows[2],
        );
        let sat_data: Vec<u64> = state.saturation_history.iter()
            .map(|v| *v as u64)
            .collect();
        f.render_widget(
            Sparkline::default()
                .data(&sat_data)
                .style(Style::default().fg(sat_color)),
            rows[3],
        );

        // Callback jitter
        let jitter_color = threshold_color(state.callback_jitter_us as f64, 500.0, 2000.0);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("Jitter: {} µs (inter-callback mean)", state.callback_jitter_us),
                Style::default().fg(jitter_color),
            )),
            rows[4],
        );
    }
}
```

Add to `src/ui/mod.rs`:

```rust
pub mod hardware_health;
pub use hardware_health::HardwareHealthPanel;
```

```bash
cargo build
```

---

## Step 6 — `IqDiagnosticsPanel` (`src/ui/iq_diagnostics.rs`)

Create `src/ui/iq_diagnostics.rs`. Shows DC offset and IQ imbalance with color coding
and a directional hint so the user knows which channel is dominant.

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct IqDiagnosticsPanel;

fn offset_color(abs_val: f32) -> Color {
    if abs_val > 0.02      { Color::Red    }
    else if abs_val > 0.005 { Color::Yellow }
    else                    { Color::Green  }
}

fn imbalance_color(abs_db: f32) -> Color {
    if abs_db > 3.0      { Color::Red    }
    else if abs_db > 1.0 { Color::Yellow }
    else                 { Color::Green  }
}

impl Panel for IqDiagnosticsPanel {
    fn name(&self) -> &'static str { "iq_diagnostics" }
    fn min_size(&self) -> (u16, u16) { (30, 6) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" IQ Diagnostics ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // DC offset
                Constraint::Length(1), // IQ imbalance value
                Constraint::Length(1), // directional hint
                Constraint::Min(0),
            ])
            .split(inner);

        let max_offset = state.dc_offset_i.abs().max(state.dc_offset_q.abs());
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "DC offset  I: {:+.4}  Q: {:+.4}",
                    state.dc_offset_i, state.dc_offset_q
                ),
                Style::default().fg(offset_color(max_offset)),
            )),
            rows[0],
        );

        let abs_imbalance = state.iq_imbalance_db.abs();
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("IQ imbalance: {:+.2} dB", state.iq_imbalance_db),
                Style::default().fg(imbalance_color(abs_imbalance)),
            )),
            rows[1],
        );

        let hint = if abs_imbalance < 1.0       { "OK — channels balanced" }
            else if state.iq_imbalance_db > 0.0 { "I channel stronger" }
            else                                { "Q channel stronger" };
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("  → {}", hint),
                Style::default().fg(Color::DarkGray),
            )),
            rows[2],
        );
    }
}
```

Add to `src/ui/mod.rs`:

```rust
pub mod iq_diagnostics;
pub use iq_diagnostics::IqDiagnosticsPanel;
```

```bash
cargo build
```

---

## Step 7 — `SystemResourcesPanel` (`src/ui/system_resources.rs`)

Create `src/ui/system_resources.rs`. Shows process CPU%, RSS memory, and the USB
throughput sparkline (which gives this panel the same live signal as the gains panel,
but in the monitoring context).

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct SystemResourcesPanel;

impl Panel for SystemResourcesPanel {
    fn name(&self) -> &'static str { "system_resources" }
    fn min_size(&self) -> (u16, u16) { (30, 10) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" System Resources ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // CPU gauge
                Constraint::Length(2), // RAM gauge
                Constraint::Length(1), // USB label
                Constraint::Min(0),    // USB sparkline
            ])
            .split(inner);

        let cpu = state.process_cpu_pct.clamp(0.0, 100.0);
        let cpu_color = if cpu > 80.0      { Color::Red    }
            else if cpu > 50.0            { Color::Yellow }
            else                          { Color::Cyan   };
        f.render_widget(
            Gauge::default()
                .label(format!("CPU  {:.1}%", cpu))
                .ratio(cpu as f64 / 100.0)
                .style(Style::default().fg(cpu_color)),
            rows[0],
        );

        let rss = state.process_rss_mb;
        // Reference ceiling: 512 MB. Gauge goes red above that, but still shows real value.
        let rss_ratio = (rss as f64 / 512.0).min(1.0);
        let rss_color = if rss_ratio > 0.8 { Color::Red } else { Color::Magenta };
        f.render_widget(
            Gauge::default()
                .label(format!("RAM  {} MB", rss))
                .ratio(rss_ratio)
                .style(Style::default().fg(rss_color)),
            rows[1],
        );

        let throughput_mb = state.current_throughput_bps as f64 / 1_000_000.0;
        f.render_widget(
            Paragraph::new(Span::raw(format!("USB  {:.2} MB/s", throughput_mb))),
            rows[2],
        );

        let sparkline_data: Vec<u64> = state.throughput_history.iter().cloned().collect();
        f.render_widget(
            Sparkline::default()
                .data(&sparkline_data)
                .style(Style::default().fg(Color::Green)),
            rows[3],
        );
    }
}
```

Add to `src/ui/mod.rs`:

```rust
pub mod system_resources;
pub use system_resources::SystemResourcesPanel;
```

```bash
cargo build
```

---

## Step 8 — Register panels, add monitoring preset, final validation

**`src/config.rs`** — add the `monitoring` preset to `LayoutConfig::default_config()`.
Both `hardware_health` and `iq_diagnostics` share the left column (50%); `telemetry`
and `system_resources` share the right column (50%). The LayoutEngine stacks panels
within the same column vertically, splitting evenly.

In `default_config()`, after inserting `minimal`:

```rust
use Position::*;
let monitoring = PresetConfig {
    panels: vec![
        PanelSpec { name: "header".into(),          position: Top,    height: Some(3), width_pct: None       },
        PanelSpec { name: "hardware_health".into(),  position: Left,   height: None,    width_pct: Some(50)   },
        PanelSpec { name: "iq_diagnostics".into(),   position: Left,   height: None,    width_pct: Some(50)   },
        PanelSpec { name: "telemetry".into(),        position: Right,  height: None,    width_pct: Some(50)   },
        PanelSpec { name: "system_resources".into(), position: Right,  height: None,    width_pct: Some(50)   },
        PanelSpec { name: "log".into(),              position: Bottom, height: Some(7), width_pct: None       },
        PanelSpec { name: "footer".into(),           position: Bottom, height: Some(3), width_pct: None       },
    ],
};
presets.insert("monitoring".into(), monitoring);
```

Keep `active_preset: "minimal"` as default — monitoring requires the new panels to be
registered, which happens only after this step. The user switches with `2`.

**`src/app.rs`** — register the three new panels after the existing registrations in
`App::new()`:

```rust
registry.register(ui::HardwareHealthPanel);
registry.register(ui::IqDiagnosticsPanel);
registry.register(ui::SystemResourcesPanel);
```

Add the `2` key handler inside the `InputMode::Normal` match arm:

```rust
KeyCode::Char('2') => {
    self.engine.set_preset("monitoring");
    self.state.lock().unwrap().push_log("Preset: monitoring");
}
```

Update the help overlay in `src/ui/overlay.rs` to include the `2` key:

```
 [2]        Preset: monitoring
```

**Final validation:**

```bash
cargo build --release   # zero errors, zero warnings
cargo test              # all tests pass
cargo clippy -- -D warnings  # zero findings
```

Manual test checklist with a real HackRF connected:

- [ ] Default view is `minimal` — identical to Phase 6
- [ ] Press `2` → switches to `monitoring` preset
  - [ ] `hardware_health` panel visible, shows green zeros at idle
  - [ ] `iq_diagnostics` panel visible, shows near-zero DC offset
  - [ ] `system_resources` panel visible, CPU% and RAM updating
- [ ] Press `Space` to start RX
  - [ ] Drop rate sparkline updates (expect 0 on clean USB)
  - [ ] ADC saturation updates (expect near 0% on normal signal levels)
  - [ ] USB throughput sparkline animates
  - [ ] IQ imbalance shows a value (may be non-zero, hardware-dependent)
- [ ] Press `1` → back to minimal preset
- [ ] Press `p` → cycles: minimal → monitoring → minimal
- [ ] All Phase 5 keys still work in both presets
