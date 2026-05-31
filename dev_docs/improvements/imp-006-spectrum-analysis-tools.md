# IMP-006 — Spectrum analysis tools

← [Home](../Home.md)

**Added:** 2026-05-30  
**Between phases:** 12 → 13

---

## Why

After the spectrum focus and tuning improvements (IMP-004, IMP-005), the panel had interactive navigation but no analytical tools. A spectrum analyzer is most useful when it can annotate, compare, and zoom — without those, it's only a passive viewer. These five tools make it possible to work with the spectrum rather than just observe it.

---

## What changed

| File | Change |
|---|---|
| `src/state.rs` | `SpectrumMarker` struct; fields: `spectrum_y_min`, `spectrum_y_max`, `spectrum_hold`, `spectrum_cursor_freq`, `spectrum_markers`, `pending_marker_freq`; `InputMode::MarkerNameInput` variant |
| `src/config.rs` | `spectrum_markers: Vec<SpectrumMarker>` in `DisplayConfig` — markers persist across restarts |
| `src/app.rs` | Key handlers: `H` (hold), `↑↓` (zoom), `J/K` (cursor), `M` (marker + name input); `InputMode::MarkerNameInput` event branch; marker save/load in `save_config` |
| `src/ui/spectrum.rs` | `BAND_PLAN` constant; Canvas: hold ghost + cursor line + marker lines; post-Canvas: band labels + marker text; indicator row: cursor info; dBFS axis tracks `spectrum_y_min/y_max` |
| `src/ui/footer.rs` | `InputMode::MarkerNameInput` case — shows frequency and name input field |

---

## Features

### 1 — Band plan overlay

Always visible. When the visible frequency range overlaps with a known band, the band name appears as a dim label at the top of the canvas at the corresponding position.

**Built-in bands:**

| Label | Range |
|---|---|
| FM | 87.5 – 108 MHz |
| VOR/ILS | 108 – 118 MHz |
| AIR | 118 – 137 MHz |
| 2m | 144 – 146 MHz |
| Marine | 156 – 174 MHz |
| WX | 162.4 – 163.3 MHz |
| 70cm | 430 – 440 MHz |
| ISM433 | 433.05 – 434.79 MHz |
| PMR | 446.0 – 446.2 MHz |
| ISM868 | 868 – 869 MHz |
| GPS-L2 | 1227.6 MHz |
| GPS-L1 | 1575.42 MHz |
| CELL | 1710 – 2170 MHz |
| 2.4G | 2400 – 2483.5 MHz |

Labels are rendered non-overlapping (closest to band center, skipped if previous label is too close).

---

### 2 — Zoom (`↑` / `↓` — spectrum focus)

Adjusts the y-axis range by moving `spectrum_y_min` while keeping `spectrum_y_max` fixed at 0 dBFS.

| Key | Effect |
|---|---|
| `↑` | Raise `y_min` by 10 dB — cuts off noise floor, signal fills more of the canvas |
| `↓` | Lower `y_min` by 10 dB — more range visible, down to −120 dBFS |

Minimum visible range: 20 dBFS (prevents collapsing to nothing). The dBFS axis labels update dynamically to always show the actual current range.

**Example progression:**

| y_min | Canvas shows | Use case |
|---|---|---|
| −120 dBFS (default) | Full range | Overview |
| −80 dBFS | −80 to 0 dBFS | Typical signal work |
| −50 dBFS | −50 to 0 dBFS | Signal detail / weak signal hunting |

---

### 3 — Hold / freeze (`H` — global)

Captures the current FFT frame as a ghost spectrum that remains visible behind the live signal.

- **Hold on:** `H` when `spectrum_hold` is `None` — snapshots `last_fft_frame.bins_dbfs`
- **Hold off:** `H` again — clears the ghost
- `[HOLD]` label appears in the panel title when active
- Ghost rendered as a dim polyline (`border_dim` color) behind the filled live spectrum
- Works outside focus mode — useful as a reference while adjusting frequency or gain

---

### 4 — Channel power cursor (`J` / `K` — spectrum focus)

A vertical line that can be moved across the spectrum to inspect signal levels at specific frequencies.

| Key | Effect |
|---|---|
| `J` | Move cursor left by `spectrum_step_hz` |
| `K` | Move cursor right by `spectrum_step_hz` |

First `J` or `K` press initializes the cursor at the current center frequency. `Esc` clears the cursor and exits focus.

When cursor is active, the tuning indicator row shows:

```
────◀  92.800 MHz  ▶────  cur: 92.750 MHz  -42.3 dBFS  J/K
```

The power value (`-42.3 dBFS`) is read from the FFT bin closest to the cursor frequency.

---

### 5 — Peak markers (`M` — spectrum focus)

Named frequency markers that persist between sessions.

**Placing a marker:**

1. Press `M` in spectrum focus
2. If cursor is active: marker placed at cursor frequency
3. If no cursor: marker placed at the highest-power bin in the current frame
4. If a marker already exists within `±spectrum_step_hz` of the target: it is **removed** (toggle behavior)
5. Otherwise: the footer opens a name input field:

```
 Marker name at 92.800 MHz:  [FM Radio▌]  [Enter] Confirm  [Esc] Cancel
```

- Type any name and press `Enter` to confirm
- Press `Enter` with an empty buffer → auto-label (`M1`, `M2`, `M3`…)
- Press `Esc` → cancel, no marker added

**On canvas:** each marker renders a vertical `status_warn`-colored line with a `▼Label` indicator on the second row of the canvas.

**Persistence:** markers are saved to `~/.config/sdrtop/config.toml` on quit (`[display] spectrum_markers`) and reloaded on startup. The TOML format:

```toml
[[display.spectrum_markers]]
freq_hz = 92800000
label = "FM Radio"

[[display.spectrum_markers]]
freq_hz = 145500000
label = "2m calling"
```

---

## Key summary (spectrum focus)

| Key | Action | Scope |
|---|---|---|
| `H` | Toggle hold | Global |
| `↑` | Zoom in (raise y_min +10 dB) | Focus |
| `↓` | Zoom out (lower y_min −10 dB) | Focus |
| `J` | Cursor left (by step) | Focus |
| `K` | Cursor right (by step) | Focus |
| `M` | Place or remove marker | Focus |
