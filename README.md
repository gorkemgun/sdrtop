# sdrtop

Real-time HackRF One monitor for the terminal. Built in Rust, inspired by btop.

If you've ever wanted to see what your radio is actually doing not just "it's receiving" but *how much* signal is dropping, whether the ADC is saturating, what the spectrum looks like right now this is the tool for that. It runs entirely in the terminal, works over SSH, and fits in a tmux pane next to your SDR pipeline.

---

## What it shows

- **Spectrum analyzer** — FFT with EMA smoothing, peak hold, noise floor, dBFS axis
- **Waterfall** — scrolling spectrogram with truecolor / 256-color / 16-color support
- **Signal metrics** — SNR, channel power (dBFS), 99% occupied bandwidth
- **Hardware health** — sample drop rate, ADC saturation, IQ imbalance, DC offset
- **RF chain** — board revision, baseband filter BW, full gain chain
- **IQ histogram** — amplitude distribution; flags saturation and dynamic range issues
- **Observer mode** — device identity and owner process when another app holds the radio
- **Six themes** — `sdr` · `nord` · `dracula` · `gruvbox` · `catppuccin` · `solarized`
- **Six layout presets** — switch on the fly with number keys

---

## Pics

| ![](/docs/pics/main.png)      | ![](/docs/pics/spectrum.png)  |
| ----------------------------- | ----------------------------- |
| ![](/docs/pics/waterfall.png) | ![](/docs/pics/spec_watf.png) |

---

## Requirements

- Linux
- HackRF One
- `libhackrf` + `pkg-config`
- Rust stable

```sh
# Arch
sudo pacman -S hackrf pkgconf

# Debian / Ubuntu
sudo apt install libhackrf-dev pkg-config
```

---

## Build & run

```sh
cargo build --release
./target/release/sdrtop
```

```sh
# Common options
sdrtop --frequency 92800000     # center frequency in Hz
sdrtop --lna 24 --vga 30        # initial gain settings
sdrtop --theme nord              # built-in theme (see list below)
sdrtop --config ~/my.toml       # custom config path
```

---

## Keys

| Key | Action |
|---|---|
| `Space` | Start / stop RX |
| `↑` / `↓` | LNA gain ±8 dB |
| `[` / `]` | VGA gain ±2 dB |
| `a` | Toggle RF amplifier |
| `f` | Enter frequency (MHz) |
| `s` | Enter sample rate (2–20 MHz) |
| `r` | Reset all settings to defaults |
| `w` | Pause / resume waterfall |
| `e` | Focus spectrum panel |
| `o` | Focus waterfall panel |
| `1`–`6` | Switch layout preset |
| `p` | Cycle presets |
| `?` | Help overlay |
| `q` | Quit and save config |

---

## Config

Saved automatically to `~/.config/sdrtop/config.toml` on quit. Hand-editing is safe.

```toml
[radio]
frequency_hz = 92800000
sample_rate  = 2000000.0
lna_gain     = 24
vga_gain     = 30
amp_enabled  = false

[display]
active_preset      = "main"
waterfall_max_rows = 64

[theme]
base = "nord"
# optional per-field overrides
# border_accent = "#88c0d0"
# value_hi      = "#ebcb8b"
```

Available themes: `sdr` (default) · `nord` · `dracula` · `gruvbox` · `catppuccin` · `solarized`

---

## Status

Phase 12 of 17 complete. Running on real hardware. Next: PortaPack / Mayhem integration.

→ [Roadmap](docs/Roadmap.md) · [Changelog](docs/CHANGELOG.md) · [Docs](docs/Home.md)

---

*Written by mustang6139 and [Claude](https://claude.ai).*
