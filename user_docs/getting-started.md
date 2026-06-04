# Getting Started

← [Back](README.md)

---

## What you need

- A Linux machine
- A HackRF One **or** an RTL-SDR dongle connected via USB
- The `libhackrf` and `librtlsdr` libraries installed

```sh
# Arch Linux / Manjaro
sudo pacman -S hackrf rtl-sdr pkgconf

# Debian / Ubuntu / Linux Mint / Pop!_OS
sudo apt install libhackrf-dev librtlsdr-dev pkg-config

# Fedora
sudo dnf install hackrf-devel rtl-sdr-devel pkgconf-pkg-config

# openSUSE Tumbleweed / Leap
sudo zypper install libhackrf-devel rtl-sdr-devel pkg-config

# Void Linux
sudo xbps-install hackrf-devel rtl-sdr-devel pkg-config

# Gentoo
sudo emerge net-wireless/hackrf net-wireless/rtl-sdr

# NixOS — add to your configuration.nix or use a dev shell:
nix-shell -p hackrf rtl-sdr pkg-config
```

> RTL-SDR support is **new** (community-contributed, confirmed on hardware) — see [Supported Hardware](hardware.md). Note that sdrtop currently links *both* libraries at build time, so install `librtlsdr` even if you only own a HackRF (and vice-versa). At runtime it's happy with whichever radio you actually plug in.

You also need Rust installed. If you don't have it yet:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Build and run

```sh
git clone https://github.com/mustang6139/sdrtop
cd sdrtop
cargo build --release
./target/release/sdrtop
```

That's it. sdrtop will find your HackRF or RTL-SDR automatically. If it doesn't, that's what the [troubleshooting](troubleshooting.md) page is for — we've all been there at 2 a.m.

---

## Common startup options

```sh
# Start tuned to a specific frequency (in Hz)
sdrtop --frequency 92800000

# Start with specific gain settings (HackRF LNA/VGA)
sdrtop --lna 24 --vga 30

# Device-agnostic primary gain (HackRF LNA / RTL-SDR tuner)
sdrtop --gain 30

# Pin a backend when you have both a HackRF and an RTL-SDR plugged in
sdrtop --device rtlsdr

# Use a different color theme
sdrtop --theme nord

# Load a custom config file
sdrtop --config ~/my-config.toml
```

---

## First run

When sdrtop starts, you may see a **device selector** if you have more than one radio connected — it lists every HackRF and RTL-SDR by type and serial. Use `↑` / `↓` (or `j` / `k`) to pick one, then press `Enter`. (Skip it entirely with `--device`.)

Once the app starts, press `Space` to begin receiving. The spectrum and waterfall will come to life. Use `↑` / `↓` to adjust the gain if the signal looks too weak or too strong — that's LNA on a HackRF, the tuner gain on an RTL-SDR.

Press `?` at any time to see the full key reference on screen.

Press `q` to quit. Your settings are saved automatically.
