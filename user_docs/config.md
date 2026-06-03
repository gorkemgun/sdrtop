# Configuration

← [Back](README.md)

---

## Where the config lives

sdrtop saves your settings automatically when you quit (`q`). The file is at:

```
~/.config/sdrtop/config.toml
```

You can open and edit it by hand — it's plain text. Changes take effect next time you start sdrtop.

---

## What's in the config

```toml
[radio]
frequency_hz = 92800000   # center frequency in Hz
sample_rate  = 2000000.0  # samples per second (2–20 million)
lna_gain     = 24         # LNA gain (0–40 dB, step 8)
vga_gain     = 30         # VGA gain (0–62 dB, step 2)
amp_enabled  = false      # RF amplifier on or off

[display]
active_preset      = "main"   # which layout to use at startup
waterfall_max_rows = 64       # how many rows of history the waterfall keeps

[theme]
base = "nord"   # which color theme to use
```

---

## Runtime input: frequency and sample rate

While sdrtop is running, you can change settings with `f` (frequency) and `s` (sample rate). See [Advanced Features](advanced.md#custom-input-modes-frequency-and-sample-rate) for input formats and examples.

---

You can save named frequency markers. They appear as vertical lines on the spectrum with a label.

```toml
[[display.spectrum_markers]]
freq_hz = 92800000
label   = "FM Radio"

[[display.spectrum_markers]]
freq_hz = 156800000
label   = "Marine ch16"
```

You can add as many as you like. You can also place them from within sdrtop using the `m` key in spectrum focus mode.

---

## Sweep scanner

The `lab_sweep` preset (`9`) and the `micro_sweep` field view scan a band wider
than one sample-rate window by retuning across it. The band and dwell time are
set in the config:

```toml
[sweep]
start_hz = 400000000   # scan from 400 MHz
stop_hz  = 500000000   # scan to 500 MHz
dwell_ms = 200         # measure each position for 200 ms (50–2000)
```

The step between positions is derived from the sample rate automatically (about
90 % of it, for a small overlap). While in the sweep panel's focus mode (`g`),
`+` / `-` nudge the dwell live, `←` / `→` move the cursor, `M` toggles peak/mean,
and `Enter` jumps the radio to the cursor frequency. Your last band and dwell are
saved on quit.

A sweep cycle takes a couple of seconds, so it's for *finding* signals, not
real-time monitoring — once you spot one, `Enter` tunes to it.

---

## Custom layout presets

A *preset* is a named arrangement of panels. sdrtop ships with built-in presets you switch between with the number keys, but you can also define your own in the config file. Your presets are merged with the built-in ones at startup, and they survive a save — sdrtop never erases hand-written presets.

**Every preset is overridable** — including the built-ins. If you define a preset with the same name as a built-in (`main`, `spectrum`, `lab_iq`, `lab_rf`, `lab_timing`, `lab_signal`, `micro_main`, …), your version replaces it, and the number key that triggers it now opens your layout. New names you invent join the `[P]` cycle automatically.

A preset is a list of panels. Each panel has a `name`, a `position`, and optionally a size:

```toml
[presets.my_view]
panels = [
  { name = "header",   position = "top",    height = 5     },
  { name = "spectrum", position = "body"                    },
  { name = "log",      position = "right",  width_pct = 30  },
  { name = "footer",   position = "bottom"                  },
]
```

**Positions:**

| Position | Where it goes | Size field |
|----------|---------------|------------|
| `top`    | Full-width strip at the top    | `height` (rows) |
| `bottom` | Full-width strip at the bottom | `height` (rows) |
| `left`   | Left column of the body        | `width_pct` (% of body) |
| `right`  | Right column of the body       | `width_pct` (% of body) |
| `body`   | Centre column (fills remaining space) | — |

**Available panel names:** `header`, `spectrum`, `waterfall`, `log`, `footer`, `signal_strip`, `rf_chain`, `iq_diagnostics`, `iq_histogram`, `hardware_health`, `signal_metrics`, `system_resources`, `timing_panel`, `sweep_panel`, `sweep_strip`, `micro_panel`, `micro_signal_panel`, `micro_gain_panel`, `micro_health_panel`, `micro_sweep_panel`.

See [Advanced Features](advanced.md#defining-custom-presets) for the full guide to creating and managing custom presets.

Quick example:

```toml
[presets.my_view]
panels = [
  { name = "header",   position = "top",    height = 2  },
  { name = "spectrum", position = "body"                 },
  { name = "log",      position = "right",  width_pct = 20 },
  { name = "footer",   position = "bottom", height = 1  },
]
```

To make it accessible via a key, name it `lab_timing`, `micro_signal`, etc. (reserved names in [Advanced Features](advanced.md#preset-system-and-layout-configuration))