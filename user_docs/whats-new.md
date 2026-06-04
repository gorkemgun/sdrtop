# What's New

← [Back](README.md)

The story of sdrtop so far — not as a wall of dates, but as **checkpoints**: the big moments where the app levelled up. Each one is condensed to the essentials.

> **Where we are now:** the interactive TUI is feature-complete, and **RTL-SDR support has just landed and works** (new — see Checkpoint 9). The ongoing work is **polishing the UI, sharpening the radio math, and squashing bugs**. So if something looks off or behaves oddly, that's exactly what we're hunting.

---

## ✅ Checkpoint 1 — It receives
The foundation: talk to the HackRF safely, pull IQ off the wire, and show it.
- Solid USB FFI layer with a clean shutdown on every exit path
- Live **spectrum analyzer** — FFT with peak hold, noise floor, dBFS and frequency axes
- Scrolling **waterfall** — truecolor / 256-color / 16-color, with a graceful fallback on basic terminals

## ✅ Checkpoint 2 — It remembers
sdrtop stopped being forgetful.
- Settings (frequency, gains, sample rate, layout) **persist** across restarts in `~/.config/sdrtop/config.toml`
- Atomic, safe saves; a missing or broken config just falls back to sane defaults
- **Six themes** (`sdr`, `nord`, `dracula`, `gruvbox`, `catppuccin`, `solarized`) and switchable **layout presets**

## ✅ Checkpoint 3 — It diagnoses
The part that makes sdrtop more than a pretty spectrum.
- **Hardware health** — drops, ADC saturation, USB errors, buffer fill, sample-rate accuracy
- **RF chain** — gain stages, frequency + wavelength, estimated **noise figure** and **minimum detectable signal**
- **IQ diagnostics** — DC offset, imbalance, **image rejection ratio**, plus an ADC amplitude **histogram**

## ✅ Checkpoint 4 — It plays nice
Less crashing, more cooperating.
- **Observer mode** — if another app already holds the radio, sdrtop watches what it can instead of falling over, then reclaims it when free
- Live **sample-rate control** (`s`) without restarting
- A big **performance overhaul** — far lower CPU/RAM at 30 fps, smooth even at high sample rates

## ✅ Checkpoint 5 — It analyzes
The spectrum and waterfall grew real tools, driven by a single highlighted **focus** key per panel.
- **Spectrum focus** (`e`) — tune with `←`/`→`, **zoom**, **hold** a ghost frame to compare, a **cursor** read-out, **band-plan** labels, and named **markers** that persist
- **Waterfall focus** (`l`) — adjustable color scale, scroll-back through history, and **frame averaging** to stretch the visible time window

## ✅ Checkpoint 6 — The lab bench
Bench-engineer views for people who care about the numbers, not just the picture.
- **Lab presets** `5`–`8`: IQ · RF · timing · signal
- Derived measurements worth trusting: **NF**, **MDS**, **IRR**, **PAPR**, sample-rate accuracy, and USB **timing/jitter** with a quality verdict
- **Hardware Vitals** now tracks sdrtop's own CPU/RAM with trend graphs
- Every lab panel marks itself **[STALE]** the instant RX stops — a frozen number is never mistaken for a live one

## ✅ Checkpoint 7 — It scans
- **Frequency sweep** (`9`) — scan a band wider than one window can show; sdrtop stitches it into one curve with band-plan labels. Focus with `g`, set the band live with `s` / `e`, and press `Enter` on a peak to tune straight to it
- **Micro field views** (`0`) — deliberately tiny single-glance read-outs (signal · gain · health · sweep) for slim splits, SSH sessions, and cyberdeck screens

## 🔧 Checkpoint 8 — Polish
The feature list is closed for now. This checkpoint is about taste: refining layout and readability, **reworking the micro view's UI**, double-checking every radio calculation, and fixing the rough edges — the groundwork that made the next leap safe to land.

## 📡 Checkpoint 9 — A second radio (you are here)
sdrtop stopped being a one-device app.
- **RTL-SDR support** (R820T / R828D / E4000) lands alongside the HackRF One, behind a clean `SdrDevice` abstraction layer — the HackRF path is untouched, the RTL path shares the same RX → FFT → UI pipeline
- The UI **adapts to the hardware**: HackRF's LNA/VGA/AMP vs RTL-SDR's single tuner gain + AGC, the right frequency and sample-rate ranges, and N/A where a measurement doesn't apply (no BB filter, no Friis NF)
- Plug in more than one radio and a **device picker** greets you at launch; `--device hackrf|rtlsdr` pins one
- **Status: working, new.** Community-contributed and confirmed on real hardware — normal RX *and* observer mode, with FM reception, tuner gain, AGC and sweep all checked out. The only open question is the zoo of RTL clones, which no single person owns. **So this is where you come in:** run it on yours and [open an issue](../../../issues) with how it went — real-world reports are what make "works" universal.

---

```
  ┌─[ sdrtop · 2026 ]──────────────────────────────────
  │  $ ./sdrtop --scan-for-hype
  │  > 0 LLMs detected in the signal path
  │  > no neural nets, no "AI-powered" sticker
  │  > just honest FFTs and a person who likes radios
  │  > carry on.
  └────────────────────────────────────────────────────
```

In a year where everything claims to be AI-powered, sdrtop is proudly powered by
math you can check yourself. The dBFS numbers are real, the bugs are mine. 📻

