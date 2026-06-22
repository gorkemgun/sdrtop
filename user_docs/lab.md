# The Lab Presets

← [Back](README.md)

sdrtop's **lab presets** are the bench-engineer views: instead of just a live spectrum, they surface the measurements sdrtop can derive about your receiver's *signal quality* and *hardware health*. They're built for setting up a clean capture and watching for trouble during a long run.

The measurements are split across four focused presets, each on its own number key:

| Key | Preset | Focus |
|-----|--------|-------|
| `5` | **Lab IQ** | IQ diagnostics + constellation + spectrum |
| `6` | **Lab RF** | RF chain (NF / MDS) + spectrum + hardware vitals |
| `7` | **Lab Timing** | stream-timing diagnostics + hardware vitals |
| `8` | **Lab Signal** | spectrum + signal metrics + waterfall |
| `9` | **Lab Sweep** | frequency scanner across a band wider than one window |

This guide explains each measurement below; the heading notes which preset to open for it. Every panel turns its border and title **[STALE]** when RX is not streaming, so you always know whether you're looking at live data or a frozen snapshot.

> The lab panels also have a focus mode for extra actions — see [Keyboard Shortcuts](keys.md#lab-panel-focus-modes). The focus key is the highlighted letter in each panel's title.

---

## RF Chain  ·  *Lab RF (`6`)*

The receiver's capability in the current configuration — what the hardware *can* do, before any signal arrives.

**Top block — what you're tuned to:**

- **Freq** — current centre frequency.
- **λ / λ/4** — the wavelength and quarter-wavelength at that frequency. Handy in the field for cutting an antenna: at 433 MHz, λ/4 ≈ 17.3 cm; at 2.4 GHz, ≈ 3.1 cm. Measure twice, cut once — copper doesn't grow back.
- **Sample rate** — the configured rate (how wide a slice of spectrum you're capturing).
- **BB filter** — the analog baseband filter bandwidth the HackRF picked for that rate.

**Gain chain:**

```
AMP[14] → LNA[24] → VGA[20] = 58 dB
```

A visual of the three amplifier stages in order, with each stage's gain and the total. The AMP stage only appears when the front-end amplifier is enabled (`a`).

**Est. NF (Friis)** — estimated cascade **Noise Figure** in dB. This is the single number that describes how much noise your receiver adds to the signal. Computed from the HackRF's known stage characteristics using the Friis formula. Lower is better:

- With AMP on at high LNA gain: ~2 dB (excellent)
- AMP off, LNA at max: ~3.5 dB (good)
- Low LNA gain: 6 dB and up (the receiver is adding significant noise)

Green below 4 dB, amber to 8 dB, red above.

**MDS** — **Minimum Detectable Signal** in dBm. The weakest signal your receiver can pull out of the noise in the current configuration:

```
MDS = −174 dBm/Hz + 10·log₁₀(bandwidth) + NF
```

A typical value at 10 MHz bandwidth with a 3.5 dB noise figure is about −100 dBm. Narrowing the BB filter or lowering the noise figure improves (lowers) the MDS. This is the number to watch when you're trying to hear something faint.

**Board / USB API** — board revision and firmware USB API version, dimmed because they're reference info, not something you monitor.

**Gain advisor + ADC utilisation gauge** (bottom) — reads the live amplitude distribution and tells you whether to raise or lower gain, with the fraction of samples landing in the ADC's sweet spot.

---

## IQ Amplitude Distribution  ·  *optional panel (`iq_histogram`)*

> In the default **Lab IQ** preset the constellation (below) now fills this slot: the same ADC data shown as a richer 2-D cloud. The histogram is still available as a panel if you want the exact Low/Mid/Clip percentages: add `iq_histogram` to a [custom layout](config.md#custom-layout-presets).

A histogram of incoming sample amplitudes across 32 bins, log-scaled vertically so both rare strong peaks and the bulk of weak samples are visible at once. Colour zones:

- **Dim (left)** — low amplitude. The ADC is under-utilised.
- **Green (centre)** — the healthy range.
- **Red (right)** — high amplitude, approaching clipping.

**Numeric breakdown** — the exact percentages so you can set gain precisely:

```
Low  12%   Mid  71%   Clip  17%
```

**PAPR** — **Peak-to-Average Power Ratio** (crest factor) in dB, estimated from the distribution. This is a quick fingerprint of *what kind* of signal you're looking at:

| PAPR | Likely signal |
|------|---------------|
| under 3 dB | CW / FM (constant envelope) |
| 3–8 dB | AM / mixed |
| 8–15 dB | wideband / spread-spectrum |
| over 15 dB | bursty / impulsive |

A status line at the bottom summarises the picture: "Dynamic range OK", "weak signal — ADC under-utilised", or "clipping risk".

---

## IQ Diagnostics  ·  *Lab IQ (`5`)*

The quality of the I/Q signal coming off the ADC. Problems here show up as artefacts in the spectrum. Each *deviation-from-ideal* is drawn as an analog **null-meter**: a centre tick is "perfect", and a coloured needle deflects left/right by how far off you are, with the span between centre and needle filled. A glance reads the state; the number beside it reads the exact value.

- **DC I / DC Q** - how far each channel is offset from zero (a null-meter each), with a combined **DC magnitude** quality bar. A high DC offset puts a fixed tone right in the middle of your spectrum.
- **DC spike** - how tall that centre-frequency spike is, in dBFS. Green below −40 dBFS.
- **Amp imbalance** - whether I and Q carry the same power (null-meter). A mismatch creates mirror images of signals on the opposite side of centre.
- **Phase imbalance** - whether I and Q are exactly 90° apart (null-meter). Also causes mirroring.
- **IRR** - **Image Rejection Ratio** in dB, as a red→green quality bar. This is the key quadrature-quality figure: it tells you how far *below* every real signal its mirror image appears. 30 dB or more is good (images are faint); below 20 dB and the images become a problem.

A contextual hint at the bottom summarises whether anything needs attention, colour-matched to severity.

---

## IQ Constellation  ·  *Lab IQ (`5`)*

The 2-D picture of the same I/Q stream, in the centre of the Lab IQ preset. Where the diagnostics give you the numbers, the constellation gives you the *shape*, and shape is often faster to read.

It plots recent I/Q sample pairs as a dot-cloud over a fixed reference frame (the unit circle, a faint ±0.5 ring, and I/Q axes). What to look for:

- A **circle** centred on the origin means healthy quadrature.
- An **ellipse** means amplitude imbalance (I and Q at different levels).
- A **tilt** means phase imbalance (I and Q not 90° apart).
- The cloud's **offset** from centre is the DC offset (a small crosshair marks the measured DC point).

The cloud is coloured by **point density**: a phosphor-scope look where sparse edges are a cool blue and the dense core glows orange, so you can see where the signal's energy actually concentrates. A measured **imbalance ellipse** is fitted over it: its axis ratio is the amplitude imbalance, its tilt the phase imbalance, the same two faults the diagnostics quantify, drawn straight onto the cloud. No live numbers sit here on purpose; they're one panel to the left. Yes, it looks like an old analog scope. That's the point.

---

## Hardware Vitals  ·  *Lab RF (`6`) / Lab Timing (`7`)*

Whether the capture chain is keeping up, with a trend sparkline under each metric.

- **Drops** — samples lost per second, plus the session total. Non-zero means USB or CPU can't keep up.
- **ADC saturation** — how often samples hit the ADC ceiling, with the session peak.
- **CPU / RAM** — sdrtop's own processor and memory use. CPU is a system-wide percentage (100% means every core is maxed), so on a multi-core machine a healthy figure is well under 100%. If CPU climbs toward the warn/crit thresholds at high sample rates, that's often the cause of drops.
- **USB errors** — zero-length USB transfers, usually a cable or hub problem. Coloured by recent rate, not session total, so a single old glitch doesn't pin it red forever.
- **SR** — configured versus actually-measured sample rate, e.g. `20.000 → 19.847 MHz (−0.8%)`. A large gap means USB can't sustain the requested rate. Shows `→ ---` when not streaming.
- **BUF fill** — receive-buffer fill percentage with history. A leading indicator: if this trends upward toward 100%, drops are about to start.

---

## Sweep  ·  *Lab Sweep (`9`)*

The HackRF sees only as much spectrum at once as the sample rate covers (±10 MHz
at 20 Msps). **Lab Sweep** maps a wider band by retuning across it: at each step
it measures briefly, records the peak and mean level, then moves on, stitching
the results into one curve with frequency on the x-axis. Known bands are labelled
from the band plan, and the cursor reads out the level and band at any point.

Because a full cycle takes a couple of seconds, sweep is for *finding* a signal,
not watching it — once you spot one, focus the panel with `g` and press `Enter`
to tune straight to the cursor frequency in normal RX. While focused, `s` / `e`
set the start / end frequency live and `+` / `-` adjust the dwell; the band and
dwell also live in the config (see [Configuration → Sweep scanner](config.md#sweep-scanner)). The
`micro_sweep` step in the `0` cycle gives the same scan as a compact field list.

---

## Using the lab presets in practice

A typical setup flow, switching presets as you go:

1. Tune to your target and start RX (`Space`).
2. In **Lab IQ (`5`)**, watch the **constellation**: adjust LNA/VGA (`↑`/`↓`, `[`/`]`) until the cloud is a bright, well-filled ring sitting comfortably *inside* the unit circle (smearing out to the edge means clipping). Glance at **IQ Diagnostics**: a centred needle on each null-meter, IRR above 30 dB and DC spike below −40 dBFS mean clean quadrature.
3. In **Lab RF (`6`)**, check the **gain advisor**, **Est. NF** and **MDS** — confirm the receiver is sensitive enough for what you're chasing.
4. In **Lab Timing (`7`)**, confirm the timing verdict is Good/Excellent before committing to a long run.
5. During a long capture, keep an eye on **Hardware Vitals** (in the `6`/`7` labs) — CPU, BUF fill, and Drops together tell you whether the run is sustainable.

---

← [Back to all screens](screens.md)
