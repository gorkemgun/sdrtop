# Phase 6 — Dashboard Engine: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)

**Goal:** Replace the fixed TUI layout with a modular panel system. Every display element
becomes a named `Panel` trait implementation stored in a `PanelRegistry`. A `LayoutEngine`
reads the active preset from `LayoutConfig` and dispatches rendering. This makes every future
panel a self-contained plugin: implement the trait, call `register()`, done.

---

## Dependency order

```
Cargo.toml              serde + toml dependencies
    ↓
src/ui/panel.rs         Panel trait — the contract every panel must satisfy
    ↓
src/ui/registry.rs      PanelRegistry — name → Box<dyn Panel>
    ↓
src/config.rs           LayoutConfig, PresetConfig, PanelSpec, Position
    ↓
src/ui/header.rs        HeaderPanel  ┐
src/ui/telemetry.rs     TelemetryPanel  │  each wraps its existing render()
src/ui/gains.rs         GainsPanel      │  function in a Panel impl
src/ui/log.rs           LogPanel        │
src/ui/footer.rs        FooterPanel  ┘
    ↓
src/ui/engine.rs        LayoutEngine — owns config + registry, drives render
    ↓
src/ui/mod.rs           remove draw(), export new types
    ↓
src/app.rs              build engine, handle p/1/2 keys, keep show_help + overlay
```

---

## Step 1 — Add serde + toml to `Cargo.toml`

Add to the `[dependencies]` section:

```toml
serde = { version = "1", features = ["derive"] }
toml  = "0.8"
```

```bash
cargo build
```

Expected: `Finished` with no errors.

---

## Step 2 — Panel trait (`src/ui/panel.rs`)

Create `src/ui/panel.rs`:

```rust
use ratatui::{layout::Rect, Frame};
use crate::state::SdrMetrics;

pub trait Panel: Send + Sync {
    fn name(&self) -> &'static str;
    fn min_size(&self) -> (u16, u16);   // (width, height) in terminal cells
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyPanel;

    impl Panel for DummyPanel {
        fn name(&self) -> &'static str { "dummy" }
        fn min_size(&self) -> (u16, u16) { (10, 3) }
        fn render(&self, _f: &mut Frame, _area: Rect, _state: &SdrMetrics) {}
    }

    #[test]
    fn panel_name_and_min_size() {
        let p = DummyPanel;
        assert_eq!(p.name(), "dummy");
        assert_eq!(p.min_size(), (10, 3));
    }
}
```

Add to `src/ui/mod.rs`:

```rust
pub mod panel;
```

```bash
cargo test ui::panel
```

Expected: `test ui::panel::tests::panel_name_and_min_size ... ok`

---

## Step 3 — PanelRegistry (`src/ui/registry.rs`)

Create `src/ui/registry.rs`:

```rust
use std::collections::HashMap;
use ratatui::{layout::Rect, Frame};
use crate::state::SdrMetrics;
use super::panel::Panel;

pub struct PanelRegistry {
    panels: HashMap<&'static str, Box<dyn Panel>>,
}

impl PanelRegistry {
    pub fn new() -> Self {
        Self { panels: HashMap::new() }
    }

    pub fn register(&mut self, panel: impl Panel + 'static) {
        self.panels.insert(panel.name(), Box::new(panel));
    }

    pub fn get(&self, name: &str) -> Option<&dyn Panel> {
        self.panels.get(name).map(|p| p.as_ref())
    }

    pub fn render_panel(&self, name: &str, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        if let Some(panel) = self.get(name) {
            panel.render(f, area, state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::panel::Panel;

    struct NamedPanel(&'static str);

    impl Panel for NamedPanel {
        fn name(&self) -> &'static str { self.0 }
        fn min_size(&self) -> (u16, u16) { (0, 0) }
        fn render(&self, _f: &mut Frame, _area: Rect, _state: &SdrMetrics) {}
    }

    #[test]
    fn register_and_retrieve() {
        let mut reg = PanelRegistry::new();
        reg.register(NamedPanel("alpha"));
        reg.register(NamedPanel("beta"));
        assert!(reg.get("alpha").is_some());
        assert!(reg.get("beta").is_some());
        assert!(reg.get("gamma").is_none());
    }
}
```

Add to `src/ui/mod.rs`:

```rust
pub mod registry;
```

```bash
cargo test ui::registry
```

Expected: `test ui::registry::tests::register_and_retrieve ... ok`

---

## Step 4 — LayoutConfig (`src/config.rs`)

Replace the entire contents of `src/config.rs` with:

```rust
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Position {
    Top,
    Bottom,
    Left,
    Right,
    Body,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PanelSpec {
    pub name: String,
    pub position: Position,
    /// Height in terminal rows — used for Top and Bottom panels.
    #[serde(default)]
    pub height: Option<u16>,
    /// Width as a percentage of the body zone — used for Left and Right panels.
    /// All panels in the same column should carry the same value; the LayoutEngine
    /// reads only the first panel's value to determine column width.
    #[serde(default)]
    pub width_pct: Option<u16>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PresetConfig {
    pub panels: Vec<PanelSpec>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LayoutConfig {
    pub active_preset: String,
    pub presets: HashMap<String, PresetConfig>,
}

impl LayoutConfig {
    pub fn default_config() -> Self {
        use Position::*;
        let minimal = PresetConfig {
            panels: vec![
                PanelSpec { name: "header".into(),    position: Top,    height: Some(3), width_pct: None },
                PanelSpec { name: "telemetry".into(), position: Body,   height: None,    width_pct: None },
                PanelSpec { name: "gains".into(),     position: Right,  height: None,    width_pct: Some(50) },
                PanelSpec { name: "log".into(),       position: Bottom, height: Some(9), width_pct: None },
                PanelSpec { name: "footer".into(),    position: Bottom, height: Some(3), width_pct: None },
            ],
        };
        let mut presets = HashMap::new();
        presets.insert("minimal".into(), minimal);
        Self { active_preset: "minimal".into(), presets }
    }

    pub fn active_panels(&self) -> &[PanelSpec] {
        self.presets
            .get(&self.active_preset)
            .map(|p| p.panels.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_minimal_preset() {
        let cfg = LayoutConfig::default_config();
        assert_eq!(cfg.active_preset, "minimal");
        assert!(!cfg.active_panels().is_empty());
    }

    #[test]
    fn active_panels_returns_correct_names() {
        let cfg = LayoutConfig::default_config();
        let names: Vec<&str> = cfg.active_panels().iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"header"));
        assert!(names.contains(&"footer"));
        assert!(names.contains(&"telemetry"));
    }

    #[test]
    fn deserialize_from_toml() {
        let raw = r#"
            active_preset = "minimal"
            [presets.minimal]
            panels = [
              { name = "header", position = "top", height = 3 },
              { name = "footer", position = "bottom", height = 3 },
            ]
        "#;
        let cfg: LayoutConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.active_panels().len(), 2);
    }
}
```

```bash
cargo test config::
```

Expected: all three config tests pass.

---

## Step 5 — Wrap existing panels with the Panel trait

Each file keeps its existing `render()` free function unchanged. A new public struct implementing `Panel` is added at the bottom of each file, and calls through to the existing function.

**`src/ui/header.rs`** — add at the bottom:

```rust
use super::panel::Panel;
use crate::state::SdrMetrics;

pub struct HeaderPanel {
    pub board_name: String,
    pub fw_version: String,
    pub serial: String,
}

impl Panel for HeaderPanel {
    fn name(&self) -> &'static str { "header" }
    fn min_size(&self) -> (u16, u16) { (40, 3) }
    fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, _state: &SdrMetrics) {
        render(f, area, &self.board_name, &self.fw_version, &self.serial);
    }
}
```

**`src/ui/telemetry.rs`** — add at the bottom:

```rust
use super::panel::Panel;
use crate::state::SdrMetrics;

pub struct TelemetryPanel {
    pub board_name: String,
    pub serial: String,
}

impl Panel for TelemetryPanel {
    fn name(&self) -> &'static str { "telemetry" }
    fn min_size(&self) -> (u16, u16) { (30, 10) }
    fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        render(f, area, state, &self.board_name, &self.serial);
    }
}
```

**`src/ui/gains.rs`** — add at the bottom:

```rust
use super::panel::Panel;
use crate::state::SdrMetrics;

pub struct GainsPanel;

impl Panel for GainsPanel {
    fn name(&self) -> &'static str { "gains" }
    fn min_size(&self) -> (u16, u16) { (20, 12) }
    fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        render(f, area, state);
    }
}
```

**`src/ui/log.rs`** — add at the bottom:

```rust
use super::panel::Panel;
use crate::state::SdrMetrics;

pub struct LogPanel;

impl Panel for LogPanel {
    fn name(&self) -> &'static str { "log" }
    fn min_size(&self) -> (u16, u16) { (20, 7) }
    fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        render(f, area, state);
    }
}
```

**`src/ui/footer.rs`** — add at the bottom.

Note: Phase 5 changed `footer::render` to take `&SdrMetrics` (for input mode display). The `Panel` impl passes state through:

```rust
use super::panel::Panel;
use crate::state::SdrMetrics;

pub struct FooterPanel;

impl Panel for FooterPanel {
    fn name(&self) -> &'static str { "footer" }
    fn min_size(&self) -> (u16, u16) { (40, 3) }
    fn render(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect, state: &SdrMetrics) {
        render(f, area, state);
    }
}
```

```bash
cargo build
```

Expected: `Finished` with no errors.

---

## Step 6 — LayoutEngine (`src/ui/engine.rs`)

The engine owns the config and registry. Its `draw()` method:
1. Splits the terminal into top / body / bottom strips
2. Renders top panels top-to-bottom
3. Renders bottom panels top-to-bottom
4. Splits the body into left / center / right columns and renders each

**Critical:** `width_pct` on body panels defines the *column* width as a share of the body zone. When multiple panels share the same column (stacked vertically), they all carry the same `width_pct` value — the engine reads only the **first** panel in that column to determine column width. Summing them would give 100% for two 50% panels, which is wrong.

Create `src/ui/engine.rs`:

```rust
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::config::{LayoutConfig, Position};
use crate::state::SdrMetrics;
use super::registry::PanelRegistry;

pub struct LayoutEngine {
    pub config: LayoutConfig,
    registry: PanelRegistry,
}

impl LayoutEngine {
    pub fn new(config: LayoutConfig, registry: PanelRegistry) -> Self {
        Self { config, registry }
    }

    pub fn active_preset(&self) -> &str {
        &self.config.active_preset
    }

    pub fn cycle_preset(&mut self) {
        let mut names: Vec<String> = self.config.presets.keys().cloned().collect();
        names.sort();
        let current = names.iter().position(|n| n == &self.config.active_preset).unwrap_or(0);
        self.config.active_preset = names[(current + 1) % names.len()].clone();
    }

    pub fn set_preset(&mut self, name: &str) {
        if self.config.presets.contains_key(name) {
            self.config.active_preset = name.to_string();
        }
    }

    pub fn draw(&self, f: &mut Frame, state: &SdrMetrics) {
        let specs = self.config.active_panels();
        let size = f.size();

        let top_specs: Vec<_> = specs.iter().filter(|s| s.position == Position::Top).collect();
        let bottom_specs: Vec<_> = specs.iter().filter(|s| s.position == Position::Bottom).collect();
        let body_specs: Vec<_> = specs.iter().filter(|s| {
            matches!(s.position, Position::Left | Position::Right | Position::Body)
        }).collect();

        let top_h: u16 = top_specs.iter().map(|s| s.height.unwrap_or(3)).sum();
        let bot_h: u16 = bottom_specs.iter().map(|s| s.height.unwrap_or(3)).sum();

        let outer = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_h),
                Constraint::Min(0),
                Constraint::Length(bot_h),
            ])
            .split(size);

        // Top panels — each gets its declared height, stacked downward
        let mut y = outer[0].y;
        for spec in &top_specs {
            let h = spec.height.unwrap_or(3);
            let area = Rect { x: outer[0].x, y, width: outer[0].width, height: h };
            self.registry.render_panel(&spec.name, f, area, state);
            y += h;
        }

        // Bottom panels — each gets its declared height, stacked downward
        let mut y = outer[2].y;
        for spec in &bottom_specs {
            let h = spec.height.unwrap_or(3);
            let area = Rect { x: outer[2].x, y, width: outer[2].width, height: h };
            self.registry.render_panel(&spec.name, f, area, state);
            y += h;
        }

        // Body — split into left / center / right columns
        if !body_specs.is_empty() {
            let left_specs: Vec<_> = body_specs.iter()
                .filter(|s| s.position == Position::Left).collect();
            let right_specs: Vec<_> = body_specs.iter()
                .filter(|s| s.position == Position::Right).collect();
            let center_specs: Vec<_> = body_specs.iter()
                .filter(|s| s.position == Position::Body).collect();

            // Column width is determined by the FIRST panel in each column.
            // All panels in the same column must carry the same width_pct value.
            let left_pct = left_specs.first()
                .and_then(|s| s.width_pct).unwrap_or(0);
            let right_pct = right_specs.first()
                .and_then(|s| s.width_pct).unwrap_or(0);

            let columns = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(left_pct),
                    Constraint::Min(0),
                    Constraint::Percentage(right_pct),
                ])
                .split(outer[1]);

            render_column(f, &left_specs, columns[0], state, &self.registry);
            render_column(f, &center_specs, columns[1], state, &self.registry);
            render_column(f, &right_specs, columns[2], state, &self.registry);
        }
    }
}

fn render_column<'a>(
    f: &mut Frame,
    specs: &[&&'a crate::config::PanelSpec],
    area: Rect,
    state: &SdrMetrics,
    registry: &PanelRegistry,
) {
    if specs.is_empty() { return; }
    let constraints: Vec<Constraint> = specs.iter().map(|_| Constraint::Min(0)).collect();
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);
    for (spec, area) in specs.iter().zip(areas.iter()) {
        registry.render_panel(&spec.name, f, *area, state);
    }
}
```

Add to `src/ui/mod.rs`:

```rust
pub mod engine;
```

```bash
cargo build
```

Expected: `Finished` with no errors.

---

## Step 7 — Wire LayoutEngine into App

**`src/ui/mod.rs`**

Remove the existing `pub fn draw(...)` function. Add module declarations and re-exports for the new types:

```rust
pub mod engine;
pub mod footer;
pub mod gains;
pub mod header;
pub mod layout;
pub mod log;
pub mod overlay;
pub mod panel;
pub mod registry;
pub mod sparkline;
pub mod spectrum;
pub mod telemetry;
pub mod waterfall;

pub use engine::LayoutEngine;
pub use footer::FooterPanel;
pub use gains::GainsPanel;
pub use header::HeaderPanel;
pub use log::LogPanel;
pub use registry::PanelRegistry;
pub use telemetry::TelemetryPanel;
```

**`src/app.rs`**

Replace the `App` struct. Remove `board_name`, `fw_version`, `serial` from the struct — they are now baked into the panel constructors. Keep `show_help`:

```rust
pub struct App {
    state: Arc<Mutex<SdrMetrics>>,
    #[allow(dead_code)]
    device: Arc<hardware::Device>,
    board_name: String,
    fw_version: String,
    serial: String,
    events: EventStream,
    show_help: bool,
    engine: ui::LayoutEngine,
}
```

In `App::new()`, build the registry and engine after the device info is read:

```rust
let config = crate::config::LayoutConfig::default_config();

let mut registry = ui::PanelRegistry::new();
registry.register(ui::HeaderPanel {
    board_name: board_name.clone(),
    fw_version: fw_version.clone(),
    serial: serial.clone(),
});
registry.register(ui::TelemetryPanel {
    board_name: board_name.clone(),
    serial: serial.clone(),
});
registry.register(ui::GainsPanel);
registry.register(ui::LogPanel);
registry.register(ui::FooterPanel);

let engine = ui::LayoutEngine::new(config, registry);
```

Initialize `engine` in the `Ok(Self { ... })` block and remove `board_name`, `fw_version`, `serial` from `App::new()`'s return — wait, keep them on the struct since they are used to build the panels. The struct still holds them for any future reference.

In `App::run()`, replace the `terminal.draw(...)` call:

```rust
terminal.draw(|f| {
    let m = self.state.lock().unwrap().clone();
    self.engine.draw(f, &m);
    if self.show_help {
        ui::overlay::render_help(f);
    }
})?;
```

Add preset key handlers inside the `InputMode::Normal` branch:

```rust
KeyCode::Char('p') => {
    self.engine.cycle_preset();
    let name = self.engine.active_preset().to_string();
    self.state.lock().unwrap().push_log(format!("Preset: {}", name));
}
KeyCode::Char('1') => {
    self.engine.set_preset("minimal");
    self.state.lock().unwrap().push_log("Preset: minimal");
}
```

The `?` key stays unchanged:

```rust
KeyCode::Char('?') => {
    self.show_help = !self.show_help;
}
```

Update the overlay help text in `src/ui/overlay.rs` to include preset keys:

```
 [P]        Cycle presets
 [1]        Preset: minimal
```

```bash
cargo build
```

Expected: `Finished` with no errors.

---

## Step 8 — Final validation

```bash
cargo build --release   # zero errors, zero warnings
cargo test              # all tests pass
cargo clippy -- -D warnings  # zero findings
```

Manual test checklist with a real HackRF connected:

- [ ] TUI renders identically to pre-Phase-6 (minimal preset matches old fixed layout)
- [ ] All Phase 5 keys still work: `q`, `Space`, `↑↓`, `[]`, `a`, `f`, `r`, `?`
- [ ] `?` — help overlay appears; press again, disappears
- [ ] `p` — cycles preset name (only `minimal` exists, stays; log shows preset name)
- [ ] `1` — switches to minimal preset, log confirms
- [ ] No crash when terminal is resized
- [ ] `cargo clippy -- -D warnings` — zero findings
