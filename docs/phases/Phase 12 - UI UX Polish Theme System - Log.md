# Phase 12 — UI/UX Polish & Theme System — Log

← [Roadmap](../Roadmap.md) | Steps: [12a](Phase%2012a%20-%20Theme%20Foundation%20-%20Steps.md) · [12b](Phase%2012b%20-%20Panel%20Visual%20Updates%20-%20Steps.md) · [12c](Phase%2012c%20-%20Header%20Footer%20Focus%20-%20Steps.md)

## Status: ✅ Done

---

## Overview

Three sub-phases implemented in sequence:

- **12a — Theme Foundation:** `Theme` struct, 6 built-ins, TOML config, `Panel::render` signature update
- **12b — Panel Visual Updates:** All panels themed + rounded; per-bin gradient spectrum; waterfall palette
- **12c — Header, Footer & Panel Focus System:** Stateless header, context-sensitive footer, 7-panel focus system, `--theme` flag

---

## Deviations from plan

### 12a — `ThemeConfig` missing `Clone`

`AppConfig` derives `Clone`, so all its fields must implement `Clone`. The newly added `ThemeConfig` was written without `Clone` in the derive list, causing a compile error on the first build. Fixed immediately by adding `Clone` to `ThemeConfig`'s derive.

### 12a — `save_config()` needed `theme` field

After `theme: ThemeConfig` was added to `AppConfig`, the existing `AppConfig { radio, display }` literal in `save_config()` stopped compiling. Fixed by adding `theme: crate::config::ThemeConfig { base: self.theme.name.to_string(), ..Default::default() }`.

### 12a — Forward declarations for 12c needed `#[allow(dead_code)]`

`focus_key()` and `focus_bindings()` were added to the `Panel` trait as default methods (returning `None` / `&[]`) so that 12c's implementation would have the hook. The compiler warned about these never being used. Suppressed with `#[allow(dead_code)]`; the annotation was removed once 12c called them.

Similarly, `board_name: String` and `serial: String` on `SdrMetrics` were added ahead of the header redesign and required `#[allow(dead_code)]`.

### 12b — ratatui 0.26 Canvas closure requires `'static` data

`Canvas::paint()` takes a `'static` closure. The spectrum panel originally tried to capture `&theme` directly inside the paint closure to color each bin — the borrow did not satisfy `'static`. **Fix:** precompute `bin_colors: Vec<ratatui::style::Color>` and the two scalar colors (`peak_hold_color`, `noise_floor_color`) outside the closure, then move them in. The closure captures only owned data.

### 12b — `WaterfallPanel` color_depth field removed

The original waterfall panel stored `color_depth: ColorDepth` detected at startup. Removed in 12b — `ColorDepth::detect()` is cheap and idempotent, so it is called at render time. This simplified the struct to zero fields.

### 12b — Standalone functions in `telemetry.rs` and `log.rs` needed theme threading

Both files had standalone `pub fn render(...)` functions called from their `Panel` impl. The `theme` parameter had to be threaded into the standalone function signature as well, not just the `Panel::render` method.

### 12c — Tasks 18+19 were atomically dependent

Adding `focused: bool` to `Panel::render` (Task 19) required updating all 14 panel `render` methods simultaneously, since the trait change broke the entire codebase in one step. This was handled by updating all callers in a single editing pass before running the compiler.

### 12c — `engine.panels_iter()` ended up unused

`LayoutEngine::panels_iter()` was added to delegate to `PanelRegistry::panels_iter()`. In `app.rs`, however, the `focus_keys` HashMap is built directly from `registry.panels_iter()` before the registry moves into the engine — so the engine-level delegating method was never called. It was removed to keep the interface clean.

### 12c — `fw_version` added to `SdrMetrics`

The header redesign reads all displayed data from `SdrMetrics`. Firmware version was not previously stored there (it was carried on the old `HeaderPanel` struct). A `fw_version: String` field was added to `SdrMetrics` and initialized in both `new_normal()` (from `device.version()`) and `new_observer()` (hardcoded `"Observer Mode"`).

### 12c — `serial` in `SdrMetrics` remains UI-unused

`TelemetryPanel` still carries its own `board_name` and `serial` struct fields (it predates the stateless header pattern). `SdrMetrics.serial` is therefore still not consumed by any panel and retains `#[allow(dead_code)]`.

### 12c — Help overlay height fix

The original `centered_rect(52, 23, ...)` produced a 23-row outer box. With `Borders::ALL`, the inner content area was 21 rows — the last two lines of the help text ("Enter confirm", "Esc cancel") were silently clipped. Corrected when expanding the overlay for 12c: new dimensions are `centered_rect(62, 32, ...)`, giving 30 visible content rows.

---

## Key decisions

### Theme struct design

Single flat `Theme` struct with named fields for every color role — no trait, no generics. Callers use `theme.border_accent`, `theme.status_ok`, etc. directly. Simple, readable, zero indirection.

### Piecewise linear gradient palette

Each theme defines a `Vec<(t, r, g, b)>` of color stops. `palette_color(t)` walks the stops and lerps between adjacent pairs. The same mechanism serves all six built-in themes without duplicating gradient math.

### Three border tiers

| Tier | Color field | Panels |
|---|---|---|
| Accent | `border_accent` | Spectrum, Waterfall |
| Default | `border_default` | RF Chain, Signal Metrics, HW Health, IQ Diag, IQ Histogram |
| Dim | `border_dim` | Gains, Log, System Resources |

Plus `border_focused` when a panel is actively focused, and `stale` / `observer` for status states.

### Per-bin spectrum gradient

Each FFT bin gets its own `Color` computed from `magnitude_to_color_themed(db, ...)`. For TrueColor terminals, this maps each bin's dBFS level to the theme's custom gradient. For 256/16-color terminals, the existing hardcoded palette is used as fallback. The `Vec<Color>` is computed once per frame outside the Canvas closure.

### Focus state lives in two places

`LayoutEngine.focused_panel: Option<String>` is the source of truth for **border rendering** — it is read by every `render_panel` call inside `draw()`.

`SdrMetrics.focused_panel` and `.focused_panel_bindings` carry the same information into the **footer panel**, which only has access to `&SdrMetrics`. When the app sets focus, it updates both simultaneously.

### `focus_keys` HashMap built before registry moves

`PanelRegistry` is consumed when passed to `LayoutEngine::new()`. The `focus_keys: HashMap<char, &'static str>` map is built by iterating `registry.panels_iter()` before the `LayoutEngine::new()` call, while the registry is still owned by `new_normal()` / `new_observer()`.

### Stateless header

`HeaderPanel` dropped all struct fields. It reads `state.board_name`, `state.fw_version`, `state.frequency`, `state.hw_streaming`, and `state.observer_mode` at render time. The header now reflects live device state rather than startup-captured strings.

### Footer context priority

```
observer_mode        → "[Q] Quit  ·  [?] Help  (Observer Mode)"
FrequencyInput       → "Frequency (MHz): [▌]  [Enter] Confirm  [Esc] Cancel"
SampleRateInput      → "Sample rate (2–20 MHz): [▌]  ..."
focused_panel set    → "[key] Desc  ·  ...  [Esc] Exit focus  — PanelName"
Normal               → standard grouped keybind list
```

---

## Files changed

### Phase 12a

| File | Change |
|---|---|
| `src/theme.rs` | **New.** `Theme` struct, 6 built-in themes, `by_name()`, `parse_hex()`, `palette_color()`, 8 unit tests |
| `src/main.rs` | `mod theme; pub use theme::Theme;` |
| `src/config.rs` | `ThemeConfig` struct, `theme` field on `AppConfig`, `build_theme()` method, 4 unit tests |
| `src/ui/panel.rs` | `render` gains `theme: &crate::Theme`; default `focus_key()` + `focus_bindings()` added |
| `src/ui/registry.rs` | `render_panel` gains `theme` parameter |
| `src/ui/engine.rs` | `draw()` + `render_column()` gain `theme` parameter |
| `src/state.rs` | `board_name: String`, `serial: String` added to `SdrMetrics` |
| `src/app.rs` | `theme: crate::Theme` field on `App`; `board_name`/`serial` in state literals; theme passed to `draw()` |

### Phase 12b

| File | Change |
|---|---|
| `src/palette.rs` | `magnitude_to_color_themed()` added; 2 unit tests |
| `src/ui/spectrum.rs` | Per-bin gradient (precomputed `Vec<Color>`); `border_accent`/`stale` theming |
| `src/ui/waterfall.rs` | Struct → zero fields; `magnitude_to_color_themed`; `border_accent`/`stale` theming |
| `src/ui/hardware_health.rs` | `threshold_color()` uses theme; `BorderType::Rounded`; `theme.border_default` |
| `src/ui/rf_chain.rs` | `theme.label/value/value_hi/status_*`; `BorderType::Rounded` |
| `src/ui/signal_metrics.rs` | `snr_color()` uses theme; stale border → `theme.stale` |
| `src/ui/iq_diagnostics.rs` | `offset_color()` + `imbalance_color()` use theme; `BorderType::Rounded` |
| `src/ui/iq_histogram.rs` | Strip colors from theme; status label from theme |
| `src/ui/gains.rs` | `theme.border_dim`; gauge fills from `theme.value_hi/value/status_ok`; `BorderType::Rounded` |
| `src/ui/telemetry.rs` | Border color from `theme.status_ok/warn`; `BorderType::Rounded` |
| `src/ui/system_resources.rs` | CPU/RAM/USB colors from theme; `BorderType::Rounded` |
| `src/ui/log.rs` | `theme.border_dim`; `BorderType::Rounded` |
| `src/ui/observer.rs` | `theme.observer`; `theme.value/value_hi/label`; `BorderType::Rounded` |

### Phase 12c

| File | Change |
|---|---|
| `src/state.rs` | `fw_version: String`, `focused_panel: Option<String>`, `focused_panel_bindings: &'static [...]` |
| `src/ui/panel.rs` | `render` gains `focused: bool`; `#[allow(dead_code)]` removed from focus methods |
| `src/ui/registry.rs` | `render_panel` gains `focused: bool`; `panels_iter()` added |
| `src/ui/engine.rs` | `focused_panel: Option<String>` field; `focus()`, `clear_focus()`, `focused_panel_name()`, `is_panel_visible()`, `get_panel_bindings()`; draw passes per-panel `focused` flag |
| `src/ui/header.rs` | Full rewrite — stateless; reads from `SdrMetrics` |
| `src/ui/footer.rs` | Full rewrite — context-sensitive (observer / input / focus / normal) |
| `src/ui/overlay.rs` | Focus keys section + `--theme` note; `centered_rect(52,23)` → `(62,32)` |
| `src/ui/spectrum.rs` | `focus_key('e')`, `focus_bindings`, `border_focused` |
| `src/ui/waterfall.rs` | `focus_key('o')`, `focus_bindings`, `border_focused` |
| `src/ui/hardware_health.rs` | `focus_key('h')`, `focus_bindings`, `border_focused` |
| `src/ui/rf_chain.rs` | `focus_key('c')`, `focus_bindings`, `border_focused` |
| `src/ui/signal_metrics.rs` | `focus_key('m')`, `focus_bindings`, `border_focused` |
| `src/ui/iq_diagnostics.rs` | `focus_key('i')`, `focus_bindings`, `border_focused` |
| `src/ui/gains.rs` | `focus_key('g')`, `focus_bindings` (LNA/VGA/AMP/Esc), `border_focused` |
| `src/app.rs` | `focus_keys: HashMap<char, &'static str>`; Esc clears focus; focus key handler; stateless `HeaderPanel`; `fw_version`/`focused_panel`/`focused_panel_bindings` in state literals |
| `src/main.rs` | `--theme <name>` CLI flag |

---

## Test results

```
running 57 tests
... all pass

cargo build  →  Finished (0 warnings)
```
