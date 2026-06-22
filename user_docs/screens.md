# What You See on Screen

← [Back](README.md)

sdrtop is divided into panels. Each panel shows a different aspect of what your radio is doing. You can switch between layout presets with the number keys `1`–`9` (and `0` for the micro field views).

---

## Command Rail

The view sdrtop opens on (`1`) — a left **instrument rail** that gathers everything a poweruser glances at, with the spectrum and waterfall bonded to its right. From top to bottom:

- **Frequency hero** — the tuned frequency in big segmented digits, the actively-tuned digit lit.
- **S-meter** — a classic analog signal-strength bar (S1…S9+60) with a green→amber→red gradient and a faint peak-hold pip, sitting under the band/sample-rate line.
- **HUNT · MONITOR · BENCH tabs** — the mode strip. The lead card below it follows what you're doing: tuning surfaces the strongest carriers (Hunt), idling shows a calm watch headline (Monitor), changing gain shows front-end health (Bench). It auto-follows your actions; in rail focus, `Tab` pins one.
- **Recall slots** — saved frequencies (`M` to store, `1`·`2`·`3` to jump), each with a little activity pip when that frequency has a signal on screen right now.
- **SIGNAL** — SNR · PWR · NF · SAT, each as a braille **oscilloscope trace** of its recent history beside the live value and a trend arrow.
- **GAIN** — AMP, LNA and VGA as ⅛-block bars (a meaning gradient while streaming), plus total gain and clip headroom.
- **STREAM** — drops, buffer fill, USB throughput, and a one-line log foot.

Press `c` to focus the rail: `←`/`→` tune, `1`·`2`·`3` recall, `M` save.

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
- **CPU / RAM** — sdrtop's own processor and memory use + trend. CPU is a system-wide percentage (100% = all cores maxed).
- **USB errors** — zero-length USB transfers, usually caused by cable or hub issues + trend.
- **SR** — configured vs. actually-measured sample rate. A large gap means USB can't sustain the requested rate.
- **BUF fill** — receive-buffer fill percentage + trend. A leading indicator — if it climbs toward 100%, drops are coming.

---

## RF chain

Diagnostic view of the signal path. Shows the current frequency and its wavelength, sample rate, baseband filter bandwidth, and a visual gain chain (AMP → LNA → VGA = total dB). Two derived figures stand out:

- **Est. NF** — estimated noise figure (how much noise the receiver adds), via the Friis formula.
- **MDS** — minimum detectable signal in dBm (the weakest signal you can hear in this configuration).

At the bottom:

- **ADC utilisation gauge** — what fraction of incoming samples land in the optimal amplitude range (not too weak, not clipping).
- **Gain advisor** — reads the ADC utilisation and tells you whether to increase or reduce gain, and by how much.

See the [lab presets guide](lab.md) for what each number means and how to use them.

---

## IQ diagnostics

Measures the quality of the I/Q signal from the ADC. The deviations-from-ideal (DC offset, amplitude and phase imbalance) are drawn as analog **null-meters** — a centre tick is "perfect", and a coloured needle deflects left or right by how far off you are, so a glance tells you the state before you read the number:

- **DC offset** — how far the I and Q channels are shifted from zero (separate null-meters), plus a combined magnitude bar. A non-zero offset causes the DC spike at the center frequency.
- **DC spike** — how tall that centre-frequency spike is, in dBFS.
- **Amplitude imbalance** — whether I and Q have the same power level. Causes mirror images in the spectrum.
- **Phase imbalance** — whether I and Q are exactly 90° apart. Also causes mirroring.
- **IRR** — image rejection ratio in dB, shown as a quality bar: how far below each real signal its mirror image appears. Higher is better (30 dB+ is clean).

A contextual hint at the bottom summarises whether anything needs attention.

---

## IQ constellation

The 2-D companion to the diagnostics, in the centre of the Lab IQ preset. It plots recent I/Q samples as a dot-cloud, so the *shape* tells the story:

- A **circle** centred on the origin is healthy quadrature.
- An **ellipse** means amplitude imbalance; a **tilt** means phase imbalance.
- The cloud's **offset** from centre is the DC offset (marked with a small crosshair).

The cloud is coloured by **density** — a phosphor-scope look where the sparse edges are a cool blue and the dense core glows orange — over a faint unit-circle and ±0.5 reference frame. A fitted **imbalance ellipse** is drawn over it: its axis ratio is the amplitude imbalance, its tilt the phase imbalance. No live numbers here on purpose — those live one panel left, in IQ diagnostics.

---

## IQ histogram

A bar chart of incoming signal amplitudes across 32 bins. The color zones show:

- **Dim (left)** — low amplitude: signal is weak, ADC is under-utilised.
- **Green (center)** — healthy range: good dynamic range usage.
- **Red (right)** — high amplitude: approaching or hitting clipping.

Below the chart: a **Low / Mid / Clip** percentage breakdown for setting gain precisely, and **PAPR** (peak-to-average power ratio) which fingerprints the signal type — under 3 dB is CW/FM, higher values mean AM, wideband, or bursty signals.

A status line tells you what it means: "Dynamic range OK", "weak signal — ADC under-utilised", or "clipping risk".

---

## Observer mode

If another app (like GNU Radio or SDR++) already has your HackRF open, sdrtop can't control it — but it doesn't crash. Instead it switches to observer mode: it reads what it can from the operating system (device info, which app is using the radio, USB stats) and displays that instead.

When the other app lets go, sdrtop picks the radio back up automatically.

---

## Layouts

Switch between preset layouts with number keys. Each preset rearranges which panels are visible and how large they are.

| Key | Layout |
|-----|--------|
| `1` | Command Rail — the instrument rail + bonded spectrum/waterfall (the default) |
| `2` | Spectrum only |
| `3` | Waterfall only |
| `4` | Spectrum + waterfall |
| `5` | Lab IQ — IQ diagnostics · constellation · spectrum ([guide](lab.md)) |
| `6` | Lab RF — RF chain · spectrum · hardware vitals ([guide](lab.md)) |
| `7` | Lab Timing — stream-timing diagnostics · hardware vitals ([guide](lab.md)) |
| `8` | Lab Signal — spectrum · signal metrics · waterfall ([guide](lab.md)) |
| `9` | Lab Sweep — frequency scanner across a wide band ([guide](lab.md)) |
| `p` | Cycle through presets |

The **lab presets** have their own detailed walkthrough: **[The Lab Presets](lab.md)**.
