# What You See on Screen

← [Back](README.md)

sdrtop is divided into panels. Each panel shows a different aspect of what your radio is doing. You can switch between layout presets with the number keys `1`–`6`.

---

## Spectrum

The main view — a live graph of signal strength across the frequency range you're tuned to. The horizontal axis is frequency, the vertical axis is signal strength (dBFS, where 0 is maximum). Stronger signals appear higher up.

- The bright line is the live signal.
- The dimmer line behind it shows the peak levels seen so far (peak hold).
- The dashed line near the bottom is the noise floor — what "silence" looks like for your radio in current conditions.

Band labels (FM, Aviation, Marine, etc.) appear at the top of the graph when relevant frequencies are in view.

---

## Waterfall

A scrolling history of the spectrum. Each new row represents one moment in time, scrolling downward. Colors go from dark (weak signal) to bright (strong signal). This lets you see patterns over time — a signal that appears and disappears, interference that comes and goes.

---

## Signal strip

A single bar at the bottom of the main view with eight live readings:

- **SNR** — signal-to-noise ratio. Higher is cleaner.
- **PWR** — channel power in dBFS.
- **NF** — estimated noise floor in dBFS.
- **SAT** — ADC saturation percentage. Non-zero means the input is clipping; turn gain down.
- **DROP** — sample drops per second. If this is non-zero, USB can't keep up.
- **BUF** — receive buffer fill percentage. A leading indicator — if this climbs toward 100%, drops are coming.
- **IQ** — IQ amplitude imbalance in dB. Small values (under ±1 dB) are normal.
- **RBW** — resolution bandwidth. Tells you the frequency resolution of the current FFT.

---

## Hardware health

Shows whether your HackRF is running smoothly, with trend sparklines for each metric:

- **Drops** — sample drops per second + session total + trend graph.
- **ADC saturation** — how often samples hit the ADC ceiling + peak + trend.
- **Jitter** — USB callback timing variance. High jitter often precedes drops.
- **USB errors** — zero-length USB transfers, usually caused by cable or hub issues + trend.

---

## RF chain

Diagnostic view of the signal path. Shows baseband filter bandwidth, total gain across all stages (LNA + VGA + AMP), board revision, USB API version, and CPLD status. At the bottom:

- **ADC utilisation gauge** — what fraction of incoming samples land in the optimal amplitude range (not too weak, not clipping).
- **Gain advisor** — reads the ADC utilisation and tells you whether to increase or reduce gain, and by how much.

---

## IQ diagnostics

Measures the quality of the I/Q signal from the ADC:

- **DC offset** — how far the I and Q channels are shifted from zero. A non-zero offset causes the DC spike at the center frequency. Shown separately for I and Q, plus a combined magnitude gauge.
- **Amplitude imbalance** — whether I and Q have the same power level. Causes mirror images in the spectrum.
- **Phase imbalance** — whether I and Q are exactly 90° apart. Also causes mirroring.

A contextual hint at the bottom summarises whether anything needs attention.

---

## IQ histogram

A bar chart of incoming signal amplitudes across 32 bins. The color zones show:

- **Dim (left)** — low amplitude: signal is weak, ADC is under-utilised.
- **Green (center)** — healthy range: good dynamic range usage.
- **Red (right)** — high amplitude: approaching or hitting clipping.

A status line below the chart tells you what it means: "Dynamic range OK", "weak signal — ADC under-utilised", or "clipping risk".

---

## Observer mode

If another app (like GNU Radio or SDR++) already has your HackRF open, sdrtop can't control it — but it doesn't crash. Instead it switches to observer mode: it reads what it can from the operating system (device info, which app is using the radio, USB stats) and displays that instead.

When the other app lets go, sdrtop picks the radio back up automatically.

---

## Layouts

Switch between preset layouts with number keys. Each preset rearranges which panels are visible and how large they are.

| Key | Layout |
|-----|--------|
| `1` | Main — spectrum + waterfall + signal strip + log |
| `2` | Spectrum only |
| `3` | Waterfall only |
| `4` | Spectrum + waterfall |
| `5` | Lab — RF chain · IQ histogram · IQ diagnostics · hardware health |
| `p` | Cycle through presets |
