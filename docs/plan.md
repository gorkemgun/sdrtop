# Project Plan: SDR-Top (HackRF Monitoring Tool)

## 1. Vision and Objective
To create a high-performance, terminal-based resource monitor (TUI) specifically for SDR devices, starting with the HackRF One and PortaPack. The tool will provide real-time insights into hardware telemetry, gain settings, and signal bandwidth usage, similar to how `btop` monitors system resources.

## 2. Technology Stack
- **Language:** Rust (Recommended for safety and TUI ecosystem)
- **TUI Framework:** `ratatui` (formerly tui-rs)
- **Hardware Interface:** `libhackrf` (via C-bindings/FFI)
- **Concurrency:** `tokio` or `crossbeam` for asynchronous telemetry polling.

## 3. Development Phases

### Phase 1: Environment & Hardware Discovery
- [x] Set up the development environment (Rust toolchain + libhackrf-dev).
- [x] Implement robust device discovery:
    - [x] Enumerate all connected HackRF devices using the library context.
    - [x] Handle "No Device Found" and "Permission Denied" errors gracefully.
    - [x] Implement basic selection logic (auto-select if single, list if multiple).
- [x] Retrieve and display basic board information:
    - [x] Extract Serial Number and Board ID.
    - [x] Fetch current Firmware version.
- [x] **Goal:** A CLI tool that prints "HackRF Found: [Serial]" or lists available devices.

### Phase 2: Telemetry Data Collection
- [x] Implement a polling loop to fetch real-time metrics:
    - [x] Sample Rate (configured vs actual).
    - [x] Center Frequency.
    - [x] Gain Settings (LNA, VGA, AMP).
    - [x] Transmit/Receive status.
- [x] Measure USB throughput (bytes per second transferred).
- [x] **Goal:** Continuous console output of hardware status.

### Phase 3: TUI Dashboard Implementation
- [ ] Create the layout using `ratatui`:
    - [ ] **Header:** Device name, Firmware, and Serial.
    - [ ] **Main Panel:** Real-time gauges for Gain and Sample Rate.
    - [ ] **Graph Panel:** Sparklines for USB transfer stability.
    - [ ] **Log Panel:** System messages/errors.
- [ ] Implement keyboard shortcuts (e.g., 'q' to quit, 'r' to reset).

### Phase 4: Signal Visualization (The "Killer Feature")
- [ ] Implement a lightweight FFT (Fast Fourier Transform) on a sample buffer.
- [ ] Create a mini-waterfall or spectrum analyzer view using Braille characters.
- [ ] Optimize the FFT thread to ensure it doesn't block the UI or drop samples.

### Phase 5: PortaPack/Mayhem Integration (Optional/Advanced)
- [ ] Explore specific telemetry if the device is running Mayhem firmware.
- [ ] Display integrated battery levels (if available via specific firmware API).

## 4. Key Challenges
- **Latency:** Ensuring the TUI remains responsive while processing high-bandwidth SDR data.
- **Cross-Platform:** Managing `libhackrf` dependencies across Linux, macOS, and Windows.
- **UI Density:** Fitting meaningful RF data into a standard terminal window size.