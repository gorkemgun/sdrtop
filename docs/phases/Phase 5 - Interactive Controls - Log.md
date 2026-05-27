# Phase 5 — Interactive Controls: Implementation Log

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Phase 5 - Interactive Controls - Steps](Phase%205%20-%20Interactive%20Controls%20-%20Steps.md)

**Status:** ✅ Complete

---

## What was built

Full keyboard control of all radio parameters. Every visible value in the TUI can
now be changed live: LNA gain, VGA gain, AMP state, and center frequency. Hardware
is called immediately on each keypress. Any error from the hardware layer appears
in the log panel — the app never crashes on a failed setter.

A frequency input mode with visual feedback, a help overlay, and a reset key that
now actually calls the hardware setters were also added.

---

## Features added in this phase

| Feature | Key | HW call |
|---|---|---|
| LNA gain +8 dB | `↑` | `set_lna_gain` |
| LNA gain −8 dB | `↓` | `set_lna_gain` |
| VGA gain +2 dB | `]` | `set_vga_gain` |
| VGA gain −2 dB | `[` | `set_vga_gain` |
| AMP toggle | `a` | `set_amp_enable` |
| Frequency input mode | `f` | `set_frequency` (on Enter) |
| Reset to defaults | `r` | all five setters |
| Help overlay | `?` | — |

---

## `InputMode` enum

Added to `src/state.rs`:

```rust
#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    FrequencyInput,
}
```

Two new fields on `SdrMetrics`:

```rust
pub input_mode: InputMode,
pub input_buf: String,
```

`input_buf` accumulates characters typed while in `FrequencyInput` mode.
`input_mode` drives the footer display and the event loop branching.

---

## Event loop restructure

The flat `match key.code` in `App::run()` was replaced with a two-level match:
first on `input_mode`, then on `key.code` within each mode. This keeps Normal
and FrequencyInput concerns completely separate.

```rust
let input_mode = self.state.lock().unwrap().input_mode.clone();
match input_mode {
    InputMode::Normal => match key.code { ... },
    InputMode::FrequencyInput => match key.code { ... },
}
```

---

## Deviation from plan: steps 2–4 and 6–8 merged

The plan described adding LNA, VGA, AMP keys (Steps 2–4) to the existing flat
match, then restructuring the whole event loop in Step 6. Doing this in two passes
would have meant writing the key handlers twice. Instead, the event loop was
restructured in a single rewrite that included all new keys at once. The end
result is identical to what the plan described.

---

## Gain clamping

LNA and VGA gains are clamped before calling the hardware setter:

| Parameter | Range | Step | Clamp |
|---|---|---|---|
| LNA | 0–40 dB | 8 dB | `(gain + 8).min(40)` / `gain.saturating_sub(8)` |
| VGA | 0–62 dB | 2 dB | `(gain + 2).min(62)` / `gain.saturating_sub(2)` |

The struct field is only updated on `Ok` — if the hardware rejects the call,
the display stays at the last confirmed value.

---

## Frequency input mode

`f` in Normal mode → `FrequencyInput`. The footer changes to show the input buffer:

```
 Frequency (MHz): [2400.000▌] | [Enter] confirm | [Esc] cancel
```

On `Enter`, the buffer is parsed as `f64` MHz and multiplied by 1 000 000 to get Hz:

```rust
let hz = (mhz * 1_000_000.0) as u64;
self.device.set_frequency(hz)?;
```

Three outcomes:
- Parse fails or value ≤ 0 → log error, stay in `FrequencyInput`
- Hardware call fails → log error, stay in `FrequencyInput` so the user can retry
- Success → update `m.frequency`, return to `Normal`, log confirmation

---

## Reset key now calls hardware

Previously `r` called `reset_to_defaults()` which only updated the struct — the
hardware kept its previous state. In Phase 5, `r` calls all five setters first,
then calls `reset_to_defaults()` to sync the struct. Any setter error is logged
individually; the reset proceeds regardless.

```rust
let results = [
    self.device.set_lna_gain(DEFAULT_LNA_GAIN),
    self.device.set_vga_gain(DEFAULT_VGA_GAIN),
    self.device.set_frequency(DEFAULT_FREQUENCY),
    self.device.set_sample_rate(DEFAULT_SAMPLE_RATE),
    self.device.set_amp_enable(false),
];
let mut m = self.state.lock().unwrap();
m.reset_to_defaults();
for r in results {
    if let Err(e) = r { m.push_log(format!("Reset error: {}", e)); }
}
```

---

## Footer

`footer::render` now takes `&SdrMetrics` instead of no state. It branches on
`input_mode` to show either the full keybinding bar or the frequency input prompt.

---

## Help overlay

`src/ui/overlay.rs` (previously a stub) now implements `render_help()`. It uses
ratatui's `Clear` widget to erase the area beneath the overlay before rendering:

```rust
f.render_widget(Clear, area);
f.render_widget(Paragraph::new(text).block(...), area);
```

The overlay is drawn last in `ui::draw()` so it appears on top of all panels.
`show_help: bool` lives on the `App` struct (not in `SdrMetrics`) because it is
UI-only state — the hardware polling task has no reason to see it.

---

## Final state

```
cargo build --release   → Finished, 0 errors, 0 warnings
cargo clippy -- -D warnings → Finished, 0 findings
```
