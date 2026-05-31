# Phase 8 — FFT Spectrum Analyzer: Log

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Steps 8a](Phase%208a%20-%20FFT%20Pipeline%20-%20Steps.md) | [Steps 8b](Phase%208b%20-%20Spectrum%20Display%20-%20Steps.md)

---

## What was built

Phase 8 was split into two independent steps files and implemented in sequence.

**Phase 8a — FFT Pipeline** builds the data path from the C callback to a processed
`FftFrame` sitting in `SdrMetrics`:

| Component | File | Role |
|---|---|---|
| `FftFrame` | `src/state.rs` | Carries bins_dbfs, peak_hold, noise_floor, freq/rate metadata, timestamp |
| `WindowFn` + `compute_window()` | `src/dsp.rs` | Hann / Hamming / Blackman windows, 4 unit tests |
| `RxContext` | `src/hardware/device.rs` | Bundles metrics Arc and sample channel sender; replaces bare pointer |
| `FftWorker` | `src/fft.rs` | Consumes IQ chunks, runs full DSP pipeline, writes FftFrame; 5 unit tests |

**Phase 8b — Spectrum Display** renders the `FftFrame` as a live panel:

| Component | File | Role |
|---|---|---|
| `SpectrumPanel` | `src/ui/spectrum.rs` | Panel impl: idle state + Canvas spectrum + axes |
| `spectrum` preset | `src/config.rs` | header → spectrum (Body, full width) → log → footer |
| `3` key | `src/app.rs` | Switches to spectrum preset |

---

## DSP pipeline (FftWorker::run)

Each iteration of the inner loop processes exactly one FFT frame worth of IQ data:

1. **Accumulate** — raw IQ bytes arrive from `rx_callback` via a crossbeam bounded
   channel (capacity 4). Chunks are appended to a local `Vec<u8>` until
   `fft_size * 2` bytes are available.

2. **Window** — each IQ pair is converted to `Complex<f32>` and multiplied by the
   Hann window coefficient. IQ byte conversion: `byte as i8 as f32 / 128.0` — the
   `i8` cast is mandatory; without it `0x80` would become `+128f32` instead of `−128f32`.

3. **FFT** — `rustfft::FftPlanner` plans once; the same plan is reused for every frame.

4. **dBFS** — `20 * log10(|z| / fft_size)`. Dividing by `fft_size` keeps the scale
   independent of transform size. Zero-magnitude bins floor to −160.0 dBFS.

5. **fftshift** — rustfft puts DC at bin 0. Rotating by N/2 centers DC:
   `shifted = [old[N/2..], old[..N/2]]`. After shift: bin 0 = lowest frequency,
   bin N/2 = DC, bin N−1 = highest frequency.

6. **EMA** — `alpha * new + (1 − alpha) * old`, alpha = 0.2. Initialized from the
   first frame; no warm-up artefact.

7. **Peak hold** — each bin decays 0.5 dB per frame, clamped to the current smoothed
   value. Result: peaks trail downward slowly, never below the live spectrum.

8. **Noise floor** — mean of the bottom 10% of EMA-smoothed bins. Used as a
   horizontal reference line in the Canvas renderer.

9. **Write** — center frequency and sample rate are read from SdrMetrics in one
   lock acquisition; a second acquisition writes the completed `FftFrame`. Two
   separate locks rather than one long one keeps the FFT compute outside the critical section.

---

## Lock discipline in rx_callback

`RxContext` replaced the bare `*const Mutex<SdrMetrics>` pointer. The callback now
follows a strict discipline:

- Lock is acquired **once** for all accumulator writes (bytes, drops, IQ sums, jitter).
- Lock is released **before** `buf.to_vec()` — the heap allocation that feeds the
  channel happens outside the critical section to avoid extending lock duration and
  risking priority inversion on the C thread.
- Channel send uses `try_send` (non-blocking); a full channel silently drops the
  chunk. The bounded capacity of 4 acts as backpressure: if FftWorker can't keep
  up, the oldest chunks are discarded rather than growing memory unboundedly.

---

## Threading model

`FftWorker` runs on `std::thread::spawn`, not `tokio::spawn`. FFT is CPU-bound;
running it inside tokio's executor would block the async scheduler and cause jitter
in the 200 ms polling task and the 100 ms UI event loop.

The channel is `crossbeam_channel::bounded(4)` — a real blocking MPSC, appropriate
for a handoff between a C thread and a Rust OS thread.

---

## Canvas rendering (SpectrumPanel)

Layout inside the panel area:

```
┌────┬──────────────────────────────────┐
│ dB │ Canvas (spectrum bars)           │
│    │                                  │
│ 6  │ Borders: TOP | RIGHT | BOTTOM    │
│cols│                                  │
│    ├──────────────────────────────────┤
│    │ freq axis labels  (1 row)        │
└────┴──────────────────────────────────┘
```

- **Green `CanvasLine`** from `(x, DB_MIN)` to `(x, bin_db)` — vertical bar per bin
- **Yellow `Points`** at `(x, peak_db)` — peak hold dot per bin
- **DarkGray `CanvasLine`** horizontal at `noise_floor` — noise reference
- **dBFS labels** (left 6 cols): five levels from 0 dBFS to −120 dBFS top to bottom,
  in a `Paragraph` with `Borders::TOP | LEFT | BOTTOM` to join visually with the canvas
- **Frequency labels** (1 row): five labels at left edge, 25%, 50%, 75%, right edge,
  computed from `center_freq_hz ± sample_rate/2`

Stale detection: if `frame.timestamp.elapsed() > 500 ms`, the panel title shows
`[STALE]` in `DarkGray`. While RX is stopped, the last frame is shown (not cleared)
but visually flagged.

---

## `#[allow(dead_code)]` annotations

Clippy `-D warnings` flagged two items at the end of Phase 8a (before 8b used them):

- `WindowFn::Hamming` and `WindowFn::Blackman` — intentionally present for future
  use, tested in `dsp::tests`, annotated on the enum rather than on each variant.
- `FftFrame` fields — not yet read by any panel; annotated on the struct. Both
  annotations were removed implicitly once `SpectrumPanel` started reading the
  fields — though in practice the `#[allow]` remains on `FftFrame` since clippy
  tracks field access at the struct level and Phase 8b reads fields through `as_ref()`.

---

## Deviations from plan

- **Steps 1–3 of Phase 8b merged into one write:** the step file split
  `SpectrumPanel` into skeleton (Step 1), canvas rendering (Step 2), and noise
  floor (Step 3). All three ended up written in a single `spectrum.rs` file
  creation since all rendering lives in `render()` and splitting would just mean
  editing the same function three times in sequence.
- **`Mutex` import in `device.rs`:** `Arc` was already absent from the original
  import list — only `Mutex` was there. Added `Arc` alongside `Mutex` in the
  `std::sync` use statement rather than a separate line.
- **"…" idle text:** used `\u{2026}` (Unicode ellipsis) rather than a literal `…`
  to keep source file ASCII-clean, consistent with Phase 7's `\u{2192}` convention.

---

## Final state

```
cargo build --release   → Finished (0 errors, 0 warnings)
cargo test              → 20 passed, 0 failed
cargo clippy -D warnings → 0 findings
```
