# Phase 6 — Dashboard Engine: Implementation Log

← [Home](../Home.md) | [Roadmap](../Roadmap.md) | [Phase 6 - Dashboard Engine - Steps](Phase%206%20-%20Dashboard%20Engine%20-%20Steps.md)

**Status:** ✅ Complete

---

## What was built

A modular panel system replacing the old hardcoded `ui::draw()` function. Every display
element is now a named `Panel` trait implementation stored in a `PanelRegistry`. A
`LayoutEngine` reads the active preset from `LayoutConfig` and dispatches rendering.
The app is functionally identical to Phase 5 (`minimal` preset = old fixed layout), but
every future panel is now a self-contained plugin: implement the trait, call `register()`.

---

## New files and their responsibilities

| File | Role |
|---|---|
| `src/ui/panel.rs` | `Panel` trait — the contract every panel must satisfy |
| `src/ui/registry.rs` | `PanelRegistry` — `HashMap<&'static str, Box<dyn Panel>>` |
| `src/ui/engine.rs` | `LayoutEngine` — reads `LayoutConfig`, builds ratatui `Rect`s, dispatches render |
| `src/config.rs` | `LayoutConfig`, `PresetConfig`, `PanelSpec`, `Position` — serde-deserializable |

---

## Panel trait

```rust
pub trait Panel: Send + Sync {
    fn name(&self) -> &'static str;
    fn min_size(&self) -> (u16, u16);
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics);
}
```

Existing panels wrapped without touching their `render()` free functions. Each gets a
public struct that delegates:

| Struct | File | Note |
|---|---|---|
| `HeaderPanel` | `ui/header.rs` | stores `board_name`, `fw_version`, `serial` at construction |
| `TelemetryPanel` | `ui/telemetry.rs` | stores `board_name`, `serial` |
| `GainsPanel` | `ui/gains.rs` | unit struct — no fields |
| `LogPanel` | `ui/log.rs` | unit struct — no fields |
| `FooterPanel` | `ui/footer.rs` | unit struct — no fields |

---

## LayoutEngine rendering model

The engine splits the terminal into three vertical zones — top, body, bottom — then splits
the body horizontally into left / center / right columns:

```
┌──────────────── Top panels (fixed height) ────────────────┐
│                                                            │
├────── Left ──────┬───── Center ─────┬────── Right ─────── ┤
│   (width_pct%)   │    (Min(0))      │   (width_pct%)      │
│                  │                  │                      │
├──────────────── Bottom panels (fixed height) ─────────────┤
│                                                            │
└────────────────────────────────────────────────────────────┘
```

`minimal` preset wires: header→Top, telemetry→Body (center), gains→Right 50%, log→Bottom, footer→Bottom.

---

## Deviation from plan: `left_pct` bug fixed

The plan computed left column width by summing `width_pct` across all left panels:

```rust
// WRONG — two 50% panels would give left_pct = 100
let left_pct: u16 = left_specs.iter().map(|s| s.width_pct.unwrap_or(50)).sum();
```

`width_pct` defines the column width, not each individual panel's share. All panels in the
same column carry the same value. The fix reads only the first panel in each column:

```rust
let left_pct = left_specs.first().and_then(|s| s.width_pct).unwrap_or(0);
let right_pct = right_specs.first().and_then(|s| s.width_pct).unwrap_or(0);
```

---

## Deviation from plan: FooterPanel passes state

The plan showed `FooterPanel::render` calling `render(f, area)` with no state argument.
Phase 5 changed `footer::render` to take `&SdrMetrics` (for `InputMode`-aware display).
The Panel impl correctly passes `state` through:

```rust
fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
    render(f, area, state);   // not render(f, area)
}
```

---

## Deviation from plan: `board_name/fw_version/serial` removed from App struct

The plan kept these fields on `App` for reference. Since they are baked into `HeaderPanel`
and `TelemetryPanel` at construction time and never read from `App` after that, they were
removed to silence the `dead_code` warning cleanly. The values live in the panels now.

---

## `layout.rs` retained as dead code

`src/ui/layout.rs` (`Chunks` struct + `build()`) was used by the old `ui::draw()`. It is
no longer called but kept with `#![allow(dead_code)]` — it may be referenced by future
phases or removed if it stays unused through Phase 8.

---

## `show_help` stays on App, not in engine

The overlay is UI-only state with no meaning to the panel system. `show_help: bool` remains
on `App` and the overlay is rendered after `engine.draw()` in the draw closure:

```rust
terminal.draw(|f| {
    self.engine.draw(f, &m);
    if self.show_help {
        ui::overlay::render_help(f);
    }
})?;
```

---

## New keys added

| Key | Action |
|---|---|
| `p` | Cycle through available presets (alphabetical order); logs active preset name |
| `1` | Jump directly to `minimal` preset |

---

## Final state

```
cargo build --release   → Finished, 0 errors, 0 warnings
cargo test              → 5 passed, 0 failed  (panel, registry, 3× config)
cargo clippy -- -D warnings → Finished, 0 findings
```
