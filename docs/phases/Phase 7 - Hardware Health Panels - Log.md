# Phase 7 — Hardware Health Panels: Log

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Steps](Phase%207%20-%20Hardware%20Health%20Panels%20-%20Steps.md)

---

## What was built

Three new `Panel` implementations plug into the Phase 6 registry:

| Panel | File | Key metric |
|---|---|---|
| `HardwareHealthPanel` | `src/ui/hardware_health.rs` | Drop rate, ADC saturation %, callback jitter |
| `IqDiagnosticsPanel` | `src/ui/iq_diagnostics.rs` | DC offset (I/Q), IQ imbalance (dB) |
| `SystemResourcesPanel` | `src/ui/system_resources.rs` | Process CPU%, RSS memory, USB throughput sparkline |

A `monitoring` preset wires all three into a two-column layout alongside `telemetry`. The `2` key switches to it; `1` returns to `minimal`.

---

## Accumulator pattern

The existing `rx_callback` (C thread) was extended with integer-only accumulation:

- **Drop detection:** `valid_length < buffer_length` → dropped pairs = `(buffer_length − valid_length) / 2`
- **Saturation detection:** bytes at `0x80` (−128i8) or `0x7F` (+127i8) hit the ADC rail
- **IQ sums:** `i_sum`, `q_sum`, `i_sq_sum`, `q_sq_sum`, `acc_sample_count` accumulated as `i64`/`u64` — no float arithmetic in the callback
- **Jitter:** `SystemTime::now()` diff between consecutive callbacks, stored as `u64` µs

The polling task (200 ms tick) snapshots all accumulators, resets them, then computes float-based derived metrics:

- **Drop rate:** `acc_drops * 1000 / elapsed_ms` (drops/sec)
- **ADC saturation %:** `acc_saturated / (acc_samples * 2) * 100`
- **IQ imbalance:** `20 * log10(i_rms / q_rms)` where `rms = sqrt(sq_sum / n)`
- **DC offset:** `mean / 128.0` normalized to −1..+1
- **Jitter:** `acc_jitter_sum / acc_jitter_count` (mean µs)

The snapshot+reset happens inside a single lock acquisition — no window where the callback could half-fill a reset accumulator.

---

## System resource task

A second `tokio::spawn` task reads `/proc/self/stat` and `/proc/self/status` every second, independent of the hardware polling task.

`/proc/self/stat` parsing uses `rsplit_once(')')` to skip past the process name field, which can contain spaces and nested parentheses. Fields after the closing paren: index 11 = utime, index 12 = stime.

CPU% is computed as `(tick_delta / ticks_per_sec / elapsed_secs) * 100`, clamped to 100%.

---

## Clippy fix: manual checked division

Clippy (`-D warnings`) flagged three `if x > 0 { ... / x }` patterns as `manual_checked_ops`. All three were converted to `checked_div`:

```rust
// Before
if elapsed_ms > 0 { m.current_throughput_bps = (bytes * 1000) / elapsed_ms; }

// After
if let Some(bps) = (bytes * 1000).checked_div(elapsed_ms) { m.current_throughput_bps = bps; }
```

The same fix was applied to `drops_per_sec` and `callback_jitter_us`.

---

## Deviations from plan

- **Drop rate computation moved:** the plan showed drop rate inside the `if elapsed_ms > 0` block. The actual code uses `checked_div` and moves the push-to-history outside that block. This matches the existing `throughput_history` pattern.
- **`→` arrow in IqDiagnosticsPanel:** plan used `→` (U+2192) as a literal character in a string literal. Used `\u{2192}` escape to keep the source file ASCII-clean.

---

## Final state

```
cargo build --release   → Finished (0 errors, 0 warnings)
cargo test              → 11 passed, 0 failed
cargo clippy -D warnings → 0 findings
```
