# Phase 12c — Header, Footer & Panel Focus System: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)
**Goal:** Redesign the header as a live status bar. Redesign the footer with grouped
keybinds that switch to context-sensitive bindings when a panel is focused. Implement
the panel focus system: each interactive panel registers a focus key; `LayoutEngine`
tracks the focused panel; `App` handles focus key events. Add `--theme` CLI flag.

**Prerequisite:** [Phase 12b](Phase%2012b%20-%20Panel%20Visual%20Updates%20-%20Steps.md) complete.
All panels rounded + themed. `cargo test` passes, zero clippy warnings.

**Sub-phases:** [12a](Phase%2012a%20-%20Theme%20Foundation%20-%20Steps.md) → [12b](Phase%2012b%20-%20Panel%20Visual%20Updates%20-%20Steps.md) → 12c

---

## Dependency order

```
src/ui/engine.rs        add focused_panel: Option<String>
    ↓
src/ui/header.rs        redesign: live status bar from SdrMetrics
    ↓
src/ui/panel.rs         focus_key() + focus_bindings() — already added in 12a as defaults
    ↓
src/ui/spectrum.rs      impl focus_key + focus_bindings
src/ui/waterfall.rs     impl focus_key + focus_bindings
src/ui/hardware_health.rs    impl focus_key + focus_bindings
src/ui/rf_chain.rs           impl focus_key + focus_bindings
src/ui/signal_metrics.rs     impl focus_key + focus_bindings
src/ui/iq_diagnostics.rs     impl focus_key + focus_bindings
src/ui/gains.rs              impl focus_key + focus_bindings
    ↓
src/ui/footer.rs        redesign: grouped + context-sensitive focus mode
    ↓
src/app.rs              focus key event handling + --theme CLI flag
    ↓
src/ui/overlay.rs       update help: focus keys table + theme name display
src/config.rs           --theme CLI flag wired
```

---

## Step 1 — `engine.rs`: `focused_panel` state

**Files:** `src/ui/engine.rs`

The `LayoutEngine` currently manages which preset is active and how panels are laid
out. Add one field for the focused panel.

- [ ] **Add `focused_panel: Option<String>` to `LayoutEngine`**:
```rust
pub struct LayoutEngine {
    pub config:        LayoutConfig,
    pub focused_panel: Option<String>,   // ← new
    // ...existing fields
}
```

- [ ] **Initialize to `None`** in `LayoutEngine::new()` or wherever the struct is constructed:
```rust
focused_panel: None,
```

- [ ] **Add helper methods** to `impl LayoutEngine`:
```rust
pub fn focus(&mut self, panel_name: &str) {
    self.focused_panel = Some(panel_name.to_string());
}

pub fn clear_focus(&mut self) {
    self.focused_panel = None;
}

pub fn is_focused(&self, panel_name: &str) -> bool {
    self.focused_panel.as_deref() == Some(panel_name)
}
```

- [ ] **Pass focused state to border rendering** — in the `draw()` / `render()` method
  of `LayoutEngine` (or wherever panel areas are resolved and panels rendered), pass
  `is_focused` information down. One way: add a parameter to the draw function or
  pass the `engine` reference itself to the draw logic so panels can check
  `engine.is_focused(self.name())`.

  The simplest approach for now: add a `focused_panel: Option<&str>` param to the
  existing draw helper, and panels receive it alongside `theme`. But since the Panel
  trait already has `theme`, the cleaner solution is to add `focused: bool` as a
  new param to `Panel::render`:

  **Alternative (recommended):** Store `focused_panel` in `SdrMetrics` so the header
  and footer can read it without needing engine access. Add to `SdrMetrics`:

```rust
pub focused_panel: Option<String>,
```

  Initialize to `None`. `App` updates this whenever `engine.focused_panel` changes,
  by syncing it into the state after each event:

```rust
// In App's event loop, after processing any key event:
if let Ok(mut m) = self.state.lock() {
    m.focused_panel = self.engine.focused_panel.clone();
}
```

  Then the `FooterPanel` and `HeaderPanel` read `state.focused_panel` to decide what
  to render. Choose whichever approach fits the existing `App` + `LayoutEngine`
  architecture better.

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 2 — `header.rs`: live status bar

**Files:** `src/ui/header.rs`

The current `HeaderPanel` stores `board_name`, `fw_version`, `serial` as struct
fields and renders static text. After this step it reads from `SdrMetrics` entirely
and renders a live status line.

- [ ] **Replace `HeaderPanel` struct** — remove the stored strings:
```rust
pub struct HeaderPanel;
```

- [ ] **Update the `Panel` impl** — remove struct field references:
```rust
impl Panel for HeaderPanel {
    fn name(&self) -> &'static str { "header" }
    fn min_size(&self) -> (u16, u16) { (60, 3) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme) {
        // Status dot
        let (dot, dot_color) = if state.observer_mode {
            ("◉", theme.observer)
        } else if state.hw_streaming {
            ("●", theme.status_ok)
        } else {
            ("○", theme.status_warn)
        };

        let status_label = if state.observer_mode {
            "OBSERVER"
        } else if state.hw_streaming {
            "LIVE"
        } else {
            "STOPPED"
        };

        // Build the status line segments
        let seg_device = format!(" {} ", state.board_name);
        let seg_status = format!("{} {}  {:.3} MHz  {:.1} Msps",
            dot, status_label,
            state.frequency as f64 / 1_000_000.0,
            state.config_sample_rate / 1_000_000.0,
        );
        let seg_gains = if state.observer_mode {
            // Show owner process if known
            state.observer_owner.as_deref()
                .map(|o| format!(" Owner: {} ", o))
                .unwrap_or_default()
        } else {
            format!(" LNA {:>2}  VGA {:>2}  AMP {}",
                state.lna_gain,
                state.vga_gain,
                if state.amp_enabled { "ON " } else { "OFF" },
            )
        };

        use ratatui::{
            layout::{Constraint, Direction, Layout},
            text::{Line, Span},
            widgets::{Block, Borders, BorderType, Paragraph},
            style::Style,
        };

        let block = Block::default()
            .title(Span::styled(
                " sdrtop ",
                Style::default().fg(theme.value_hi),
            ))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_dim));
        let inner = block.inner(area);
        f.render_widget(block, area);

        // Single line: [device] · [● LIVE  freq  rate] · [gains]
        let line = Line::from(vec![
            Span::styled(&seg_device, Style::default().fg(theme.label)),
            Span::styled(" · ", Style::default().fg(theme.border_dim)),
            Span::styled(dot, Style::default().fg(dot_color)),
            Span::raw(" "),
            Span::styled(status_label, Style::default().fg(dot_color)),
            Span::raw("  "),
            Span::styled(
                format!("{:.3} MHz", state.frequency as f64 / 1_000_000.0),
                Style::default().fg(theme.value_hi),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{:.1} Msps", state.config_sample_rate / 1_000_000.0),
                Style::default().fg(theme.value),
            ),
            Span::styled(" · ", Style::default().fg(theme.border_dim)),
            Span::styled(&seg_gains, Style::default().fg(theme.value)),
        ]);

        f.render_widget(Paragraph::new(line), inner);
    }
}
```

- [ ] **Update all `HeaderPanel` construction sites in `app.rs`** — since it no longer
  takes constructor params, replace:
```rust
// BEFORE:
registry.register(HeaderPanel { board_name: ..., fw_version: ..., serial: ... });

// AFTER:
registry.register(HeaderPanel);
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Visual check:** start the app (`cargo run`). The header should show the live
  status with device name, `○ STOPPED`, frequency, sample rate, and gains in one line.
  Press `Space` to start RX — the dot should turn green and become `● LIVE`.

---

## Step 3 — Focus keys on panels

**Files:** `src/ui/spectrum.rs`, `src/ui/waterfall.rs`, `src/ui/hardware_health.rs`,
`src/ui/rf_chain.rs`, `src/ui/signal_metrics.rs`, `src/ui/iq_diagnostics.rs`,
`src/ui/gains.rs`

For each panel, add `focus_key()` and `focus_bindings()` implementations after the
existing `min_size()` method.

- [ ] **`SpectrumPanel`:**
```rust
fn focus_key(&self) -> Option<char> { Some('e') }
fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
    &[
        ("[Esc]",  "Exit focus"),
        ("[+/-]",  "dB scale adjust"),
        ("[N]",    "Toggle noise floor line"),
    ]
}
```

- [ ] **`WaterfallPanel`:**
```rust
fn focus_key(&self) -> Option<char> { Some('o') }
fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
    &[
        ("[Esc]",  "Exit focus"),
        ("[W]",    "Pause / resume"),
        ("[+/-]",  "Contrast"),
    ]
}
```

- [ ] **`HardwareHealthPanel`:**
```rust
fn focus_key(&self) -> Option<char> { Some('h') }
fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
    &[
        ("[Esc]",  "Exit focus"),
        ("[C]",    "Reset drop counter"),
    ]
}
```

- [ ] **`RfChainPanel`:**
```rust
fn focus_key(&self) -> Option<char> { Some('c') }
fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
    &[
        ("[Esc]",  "Exit focus"),
        ("[↑↓]",  "LNA gain"),
        ("[[]]",   "VGA gain"),
        ("[A]",    "AMP toggle"),
        ("[F]",    "Frequency"),
        ("[S]",    "Sample rate"),
    ]
}
```

- [ ] **`SignalMetricsPanel`:**
```rust
fn focus_key(&self) -> Option<char> { Some('m') }
fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
    &[("[Esc]", "Exit focus")]
}
```

- [ ] **`IqDiagnosticsPanel`:**
```rust
fn focus_key(&self) -> Option<char> { Some('i') }
fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
    &[("[Esc]", "Exit focus")]
}
```

- [ ] **`GainsPanel`:**
```rust
fn focus_key(&self) -> Option<char> { Some('g') }
fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
    &[
        ("[Esc]",  "Exit focus"),
        ("[↑↓]",  "LNA gain"),
        ("[[]]",   "VGA gain"),
        ("[A]",    "AMP toggle"),
    ]
}
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 4 — `footer.rs`: grouped + context-sensitive

**Files:** `src/ui/footer.rs`

The footer needs to know which panel is focused to switch its content. It reads
`state.focused_panel` (added in Step 1).

- [ ] **Replace the `render` function** entirely:

```rust
use ratatui::{
    layout::{Alignment, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, BorderType, Paragraph},
    Frame,
};

use crate::state::{InputMode, SdrMetrics};
use crate::ui::panel::Panel;

pub struct FooterPanel;

impl Panel for FooterPanel {
    fn name(&self) -> &'static str { "footer" }
    fn min_size(&self) -> (u16, u16) { (40, 3) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme) {
        let sep = Span::styled(" · ", Style::default().fg(theme.border_dim));
        let key = |s: &'static str| Span::styled(s, Style::default().fg(theme.value_hi));
        let lbl = |s: &'static str| Span::styled(s, Style::default().fg(theme.label));

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme.border_dim));

        // Observer mode
        if state.observer_mode {
            let line = Line::from(vec![
                Span::styled("Observer Mode — Hardware controls disabled", Style::default().fg(theme.observer)),
                sep.clone(),
                key("[?]"), lbl(" Help"),
                sep.clone(),
                key("[Q]"), lbl(" Quit"),
            ]);
            f.render_widget(
                Paragraph::new(line).block(block).alignment(Alignment::Center),
                area,
            );
            return;
        }

        // Input modes
        match &state.input_mode {
            InputMode::FrequencyInput => {
                let line = Line::from(vec![
                    lbl("Frequency (MHz): "),
                    Span::styled(format!("{}▌", state.input_buf), Style::default().fg(theme.value_hi)),
                    sep.clone(),
                    key("[Enter]"), lbl(" confirm"),
                    sep.clone(),
                    key("[Esc]"), lbl(" cancel"),
                ]);
                f.render_widget(
                    Paragraph::new(line).block(block).alignment(Alignment::Center),
                    area,
                );
                return;
            }
            InputMode::SampleRateInput => {
                let line = Line::from(vec![
                    lbl("Sample rate (2–20 MHz): "),
                    Span::styled(format!("{}▌", state.input_buf), Style::default().fg(theme.value_hi)),
                    sep.clone(),
                    key("[Enter]"), lbl(" confirm"),
                    sep.clone(),
                    key("[Esc]"), lbl(" cancel"),
                ]);
                f.render_widget(
                    Paragraph::new(line).block(block).alignment(Alignment::Center),
                    area,
                );
                return;
            }
            InputMode::Normal => {}
        }

        // Panel focus mode
        if let Some(ref focused) = state.focused_panel {
            // Title shows which panel is focused
            let title = format!(" {} focus ", focused);
            let focused_block = block.title(Span::styled(
                title,
                Style::default().fg(theme.border_focused),
            ));

            // Bindings come from the panel's focus_bindings() — but FooterPanel
            // doesn't have access to the panel registry here. Solution: read from
            // state.focused_panel_bindings (a &'static [(&'static str, &'static str)]
            // stored in SdrMetrics when focus changes in app.rs).
            //
            // If focused_panel_bindings is stored in state (see Step 5), render them:
            let mut spans: Vec<Span> = Vec::new();
            for (i, (k, desc)) in state.focused_panel_bindings.iter().enumerate() {
                if i > 0 { spans.push(sep.clone()); }
                spans.push(key(k));
                spans.push(lbl(&format!(" {}", desc)));
            }

            f.render_widget(
                Paragraph::new(Line::from(spans))
                    .block(focused_block)
                    .alignment(Alignment::Center),
                area,
            );
            return;
        }

        // Normal mode — grouped keybinds
        let line = Line::from(vec![
            key("[SPACE]"), lbl(" RX"),
            sep.clone(),
            key("[↑↓]"), lbl(" LNA"),
            Span::raw(" "),
            key("[[]]"), lbl(" VGA"),
            Span::raw(" "),
            key("[A]"), lbl(" AMP"),
            sep.clone(),
            key("[F]"), lbl(" Freq"),
            Span::raw(" "),
            key("[S]"), lbl(" Rate"),
            sep.clone(),
            key("[?]"), lbl(" Help"),
            Span::raw(" "),
            key("[Q]"), lbl(" Quit"),
        ]);
        f.render_widget(
            Paragraph::new(line).block(block).alignment(Alignment::Center),
            area,
        );
    }
}
```

- [ ] **Add `focused_panel_bindings` to `SdrMetrics`** in `src/state.rs` — the footer
  reads these from state rather than needing panel registry access:

```rust
pub focused_panel_bindings: &'static [(&'static str, &'static str)],
```

Initialize in `App`:
```rust
focused_panel_bindings: &[],
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 5 — `app.rs`: focus key event handling

**Files:** `src/app.rs`

The `App` event loop currently handles keystrokes in a `match` on `KeyCode`. Add
focus key detection before the existing hardware control handling.

- [ ] **Build a focus key map at startup** — a map from `char` → panel name, built
  from the registered panels. This avoids hardcoding the map in the event loop:

```rust
// In App::new() or App::new_normal(), after registering all panels:
let focus_keys: std::collections::HashMap<char, &'static str> = registry
    .panels()                        // returns iter of &Box<dyn Panel>
    .filter_map(|p| p.focus_key().map(|k| (k, p.name())))
    .collect();
```

Store `focus_keys` on `App` as `focus_keys: std::collections::HashMap<char, &'static str>`.

- [ ] **Add focus key handling** in the key event dispatch — insert BEFORE the
  existing hardware key match arms, and only when in `InputMode::Normal`:

```rust
// Focus key detection (only in normal mode, not observer mode)
if !observer_mode {
    if let KeyCode::Char(c) = key_code {
        if let Some(&panel_name) = self.focus_keys.get(&c) {
            // Is this panel visible in the current preset?
            if self.engine.is_panel_visible(panel_name) {
                self.engine.focus(panel_name);
                // Find the panel's bindings and store in state
                let bindings = self.registry
                    .get(panel_name)
                    .map(|p| p.focus_bindings())
                    .unwrap_or(&[]);
                if let Ok(mut m) = self.state.lock() {
                    m.focused_panel = Some(panel_name.to_string());
                    m.focused_panel_bindings = bindings;
                }
                continue; // don't process further
            }
        }
    }
}

// Esc clears focus if focused
if key_code == KeyCode::Esc && self.engine.focused_panel.is_some() {
    self.engine.clear_focus();
    if let Ok(mut m) = self.state.lock() {
        m.focused_panel = None;
        m.focused_panel_bindings = &[];
    }
    continue;
}
```

- [ ] **Sync focused panel border** — when drawing, pass the focused panel name so
  panels can detect focus and use `theme.border_focused`. The cleanest way: in the
  draw loop, check `engine.is_focused(panel.name())` and override the border color.
  Since the Panel trait doesn't currently know if it's focused, pass a `bool focused`
  alongside the existing params.

  **Pragmatic alternative:** add `focused: bool` to `Panel::render`:

```rust
fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool);
```

  All panels: add `_focused: bool` as the last param. Panels that care about focus
  (spectrum, waterfall, hardware_health, rf_chain, signal_metrics, iq_diagnostics,
  gains) use it to switch their border color:

```rust
.border_style(Style::default().fg(
    if focused { theme.border_focused } else { theme.border_default }
))
```

  In the draw call site:
```rust
let focused = engine.is_focused(panel.name());
panel.render(f, area, &state, &theme, focused);
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Manual test:** start the app. Press `e` — the Spectrum panel border should
  brighten to `border_focused`. The footer should change to show spectrum-specific
  bindings. Press `Esc` — both return to normal.

---

## Step 6 — `--theme` CLI flag

**Files:** `src/config.rs` (clap struct), `src/app.rs`

- [ ] **Add `--theme` to the clap arg struct** in `src/config.rs`:

```rust
#[derive(Parser, Debug)]
pub struct CliArgs {
    // ...existing args...
    /// Color theme. One of: sdr, nord, dracula, gruvbox, catppuccin, solarized.
    #[arg(long)]
    pub theme: Option<String>,
}
```

- [ ] **Apply `--theme` override** in `App::new()` after loading config — if
  `cli_args.theme` is Some, override `config.theme.base`:

```rust
if let Some(ref name) = cli_args.theme {
    config.theme.base = name.clone();
}
let theme = config.build_theme();
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

- [ ] **Test:** `cargo run -- --theme nord` should launch with the Nord palette.
  `cargo run -- --theme invalid_name` should launch with the sdr default (no crash).

---

## Step 7 — `overlay.rs`: update help screen

**Files:** `src/ui/overlay.rs`

The help screen currently lists presets `[1]`–`[6]`. Add the new panel focus keys
and note the `--theme` flag.

- [ ] **Add focus keys section** to the help text — insert after the waterfall line:

```rust
" [E]        Focus: Spectrum\n\
 [O]        Focus: Waterfall\n\
 [H]        Focus: Hardware Health\n\
 [C]        Focus: RF Chain\n\
 [M]        Focus: Signal Metrics\n\
 [I]        Focus: IQ Diagnostics\n\
 [G]        Focus: Gains\n\
 [Esc]      Exit panel focus\n\
```

- [ ] **Increase the overlay height** in `centered_rect` call to accommodate the
  new lines (add 8 to the current height value).

- [ ] **Add theme note** at the bottom of the help text:

```rust
"\n\
 Theme: set in config.toml [theme] base = \"nord\"  or  --theme nord\
```

- [ ] **Run `cargo build`**. Expected: `Finished`.

---

## Step 8 — Final validation

```bash
cargo build --release
cargo test
cargo clippy -- -D warnings
```

Expected: zero errors, zero warnings, all tests pass.

**Manual test checklist:**

- [ ] App starts with `sdr` default theme — deep black, cyan borders, orange frequency
- [ ] `[1]`–`[6]` all presets: rounded corners on all panels, no plain square corners
- [ ] Spectrum shows gradient bars (blue/cyan cold → orange/red hot) instead of solid green
- [ ] Waterfall color matches spectrum cold/hot gradient
- [ ] Header: `○ STOPPED` → press Space → `● LIVE  433.920 MHz  10.0 Msps  LNA 16  VGA 20  AMP OFF`
- [ ] Footer: grouped keybinds with `·` separators
- [ ] Press `e` → Spectrum panel border brightens, footer shows spectrum bindings
- [ ] Press `h` → Hardware Health border brightens, footer shows health bindings
- [ ] Press `c` → RF Chain border brightens, `[↑↓] LNA  [[] VGA  [A] AMP  [F] Freq  [S] Rate` in footer
- [ ] Press `Esc` → focus cleared, normal footer restored
- [ ] Press `?` → help overlay shows focus keys section
- [ ] `cargo run -- --theme nord` → arctic blue palette
- [ ] `cargo run -- --theme gruvbox` → warm amber/olive palette
- [ ] `cargo run -- --theme dracula` → purple accent
- [ ] `cargo run -- --theme catppuccin` → soft mauve palette
- [ ] `cargo run -- --theme solarized` → Solarized Dark palette
- [ ] Edit `~/.config/sdrtop/config.toml`, add `[theme]\nbase = "gruvbox"`, restart → persists
- [ ] Quit and relaunch — theme setting persists in config
- [ ] All Phase 5–11 keys still work: `[F]`, `[S]`, `[R]`, `[A]`, `[↑↓]`, `[[] []]`
- [ ] Observer mode (if available): header shows `◉ OBSERVER`, footer shows observer text
