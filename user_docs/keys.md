# Keyboard Shortcuts

← [Back](README.md)

---

## General

| Key | What it does |
|-----|-------------|
| `Space` | Start or stop receiving |
| `f` | Type a new center frequency (in MHz) |
| `s` | Type a new sample rate (HackRF 2–20 MHz · RTL-SDR 0.9–3.2 MHz) |
| `r` | Reset all settings to defaults |
| `a` | Toggle the RF amplifier (HackRF) / tuner AGC (RTL-SDR) |
| `w` | Pause or resume the waterfall |
| `h` | Freeze the spectrum (hold the current frame) |
| `e` | Enter spectrum focus mode |
| `l` | Enter waterfall focus mode |
| `c` | Enter Command Rail focus mode (in the `1` preset) |
| `1` / `2` / `3` / `4` | Layout presets — Command Rail (default) · spectrum · waterfall · both |
| `5` / `6` / `7` / `8` / `9` | Lab presets — IQ, RF, timing, signal, sweep (specialised diagnostics layouts) |
| `0` | Micro field-mode view — compact layout for small screens / SSH; press again to cycle (signal → gain → health → sweep) |
| `p` | Cycle through presets |
| `Tab` | Show or hide the footer bar |
| `?` | Show the help overlay |
| `q` | Quit and save settings |

---

## Gain

| Key | What it does |
|-----|-------------|
| `↑` / `↓` | Primary gain up or down — HackRF LNA (±8 dB) / RTL-SDR tuner (next table step) |
| `[` / `]` | VGA gain up or down by 2 dB (HackRF only) |

On a **HackRF**, LNA (Low Noise Amplifier) is the first gain stage — how much you amplify before the signal reaches the chip — and VGA (Variable Gain Amplifier) is the second stage, fine-tuning the level further in. A good starting point: LNA 24, VGA 30.

On an **RTL-SDR** there's a single tuner gain that steps through a fixed table of values (the `↑`/`↓` keys walk it), and no VGA — so `[`/`]` simply do nothing. Instead of a VGA you have tuner **AGC**, toggled with `a`.

Either way: if the spectrum is maxed out (everything near 0 dBFS), turn it down. If it's all noise at the bottom, try turning it up.

---

## Spectrum focus mode

Press `e` to enter focus mode on the spectrum panel. The border changes color to show you're in focus mode.

| Key | What it does |
|-----|-------------|
| `←` / `→` | Tune the center frequency by one step |
| `[` / `]` | Change the tuning step size (1 kHz up to 10 MHz) |
| `↑` / `↓` | Zoom the dBFS axis (expand or compress the signal range shown) |
| `j` / `k` | Move the cursor left or right across the spectrum |
| `m` | Place a named marker at the cursor position |
| `b` | Cycle channel bandwidth on the nearest marker |
| `h` | Hold / unhold spectrum frame (freeze behind live signal) |
| `Esc` | Exit focus mode |

---

## Waterfall focus mode

Press `l` to enter focus mode on the waterfall panel.

| Key | What it does |
|-----|-------------|
| `↑` / `↓` | Adjust the color scale (show faint or strong signals in more detail) |
| `[` / `]` | Frame averaging — combine multiple frames per row for a longer time window |
| `+` / `-` | Frequency zoom — magnify the centre of the band (`=` also zooms in) |
| `m` | Place or remove a frequency cursor |
| `←` / `→` | Move the cursor frequency when one is placed |
| `j` / `k` | Scroll back and forth through waterfall history |
| `Esc` | Exit focus mode |

---

## Command Rail focus mode

Press `c` to drive the Command Rail (the `1` preset). The border highlights and the footer lists the keys; `Esc` exits.

| Key | What it does |
|-----|-------------|
| `←` / `→` | Tune the center frequency by one step (auto-switches the mode strip to Hunt) |
| `1` / `2` / `3` | Jump to recall slot 1, 2 or 3 |
| `M` | Save the current tuning to the next recall slot |
| `Tab` | Cycle the HUNT · MONITOR · BENCH mode manually (otherwise it auto-follows your actions) |
| `L` | Toggle the full-log overlay |
| `Esc` | Close the log overlay if open, otherwise exit focus mode |

---

## Lab panel focus modes

Some diagnostics panels in the lab presets support a focus mode that adds a few
panel-specific actions. Each focusable panel shows its focus key as a
**highlighted letter in its title** (e.g. the **I** in "**I**Q Diagnostics") —
press that key to enter. While focused the border highlights and the footer
lists the extra keys; `Esc` exits.

| Key | Panel | What it adds |
|-----|-------|--------------|
| `i` | **I**Q Diagnostics (`[5]` lab_iq) | `C` — log a snapshot of the current DC offset, IQ imbalance and phase |
| `v` | Hardware **V**itals (`[6]` lab_rf / `[7]` lab_timing) | `R` — reset the session drop counter · `C` — clear the trend sparklines |
| `t` | **T**iming (`[7]` lab_timing) | `R` — reset the session jitter peak · `C` — clear the jitter / throughput history |
| `g` | Sweep `[G]` (`[9]` lab_sweep) | `←/→` — move cursor · `s` / `e` — set start / end frequency · `M` — peak/mean curve · `+/-` — dwell time · `Enter` — tune to the cursor frequency |

The RF chain has no separate focus mode — its gain controls are the regular
global keys (`↑`/`↓` LNA, `[`/`]` VGA, `a` AMP, `r` reset), which work everywhere.

---

## Tips

- If you're not sure what a reading means, the `?` overlay shows a quick summary while you use the app.
- Gain settings and frequency are saved when you quit with `q`. You can also edit them directly in the config file.
