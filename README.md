# sdrtop

A btop-inspired TUI for monitoring HackRF and PortaPack SDR devices.

## Prerequisites

### Arch Linux
```bash
sudo pacman -S hackrf pkgconf base-devel
```

### Linux (Debian/Ubuntu)
```bash
sudo apt install libhackrf-dev pkg-config
```

### macOS
```bash
brew install hackrf pkg-config
```

## Building

```bash
cargo build --release
```

## Usage
```bash
./target/release/sdrtop
```