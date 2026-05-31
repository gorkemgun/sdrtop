# Phase 5 — Interactive Controls: Steps

← [Home](../Home.md) | [Roadmap](../Roadmap.md)

**Goal:** Every parameter visible in the TUI can be changed live from the keyboard.
Hardware is called immediately on each keypress; the display reflects the new value
within one render frame. Any hardware error appears in the log panel — the app never crashes.

---

## Dependency order

```
state.rs          InputMode enum + input_mode / input_buf fields on SdrMetrics
    ↓
app.rs            event loop matches new keys + calls device setters
    ↓
ui/footer.rs      reads InputMode from SdrMetrics to show context-sensitive hints
    ↓
ui/overlay.rs     help overlay rendered on top when show_help = true
```

---

## Step 1 — `InputMode` in `src/state.rs`

Add the mode enum and two new fields to `SdrMetrics`:

```rust
#[derive(Clone, PartialEq)]
pub enum InputMode {
    Normal,
    FrequencyInput,
}
```

Add to `SdrMetrics`:

```rust
pub input_mode: InputMode,
pub input_buf: String,   // accumulates characters typed in FrequencyInput mode
```

Initialize in `App::new()` (in `app.rs`):

```rust
input_mode: InputMode::Normal,
input_buf: String::new(),
```

`cargo build` must pass before moving on.

---

## Step 2 — LNA gain keys (`↑` / `↓`)

**File:** `src/app.rs`

Add `show_help: bool` to the `App` struct (needed in later steps):

```rust
pub struct App {
    state: Arc<Mutex<SdrMetrics>>,
    #[allow(dead_code)]
    device: Arc<hardware::Device>,
    board_name: String,
    fw_version: String,
    serial: String,
    events: EventStream,
    show_help: bool,   // ← add this
}
```

Initialize `show_help: false` in `App::new()`.

In `App::run()`, inside the `Normal` input mode branch of the key handler, add:

```rust
KeyCode::Up => {
    let (gain, result) = {
        let m = self.state.lock().unwrap();
        let new_gain = (m.lna_gain + 8).min(40);
        let result = self.device.set_lna_gain(new_gain);
        (new_gain, result)
    };
    let mut m = self.state.lock().unwrap();
    match result {
        Ok(()) => {
            m.lna_gain = gain;
            m.push_log(format!("LNA gain → {} dB", gain));
        }
        Err(e) => m.push_log(format!("LNA gain error: {}", e)),
    }
}
KeyCode::Down => {
    let (gain, result) = {
        let m = self.state.lock().unwrap();
        let new_gain = m.lna_gain.saturating_sub(8);
        let result = self.device.set_lna_gain(new_gain);
        (new_gain, result)
    };
    let mut m = self.state.lock().unwrap();
    match result {
        Ok(()) => {
            m.lna_gain = gain;
            m.push_log(format!("LNA gain → {} dB", gain));
        }
        Err(e) => m.push_log(format!("LNA gain error: {}", e)),
    }
}
```

Note: the lock is dropped before re-acquiring it to avoid deadlock. Compute new value and call hardware while holding the lock for the read, then re-lock to write.

`cargo build` must pass before moving on.

---

## Step 3 — VGA gain keys (`[` / `]`)

**File:** `src/app.rs`

Same pattern as LNA. Add in the `Normal` mode key handler:

```rust
KeyCode::Char('[') => {
    let (gain, result) = {
        let m = self.state.lock().unwrap();
        let new_gain = m.vga_gain.saturating_sub(2);
        let result = self.device.set_vga_gain(new_gain);
        (new_gain, result)
    };
    let mut m = self.state.lock().unwrap();
    match result {
        Ok(()) => {
            m.vga_gain = gain;
            m.push_log(format!("VGA gain → {} dB", gain));
        }
        Err(e) => m.push_log(format!("VGA gain error: {}", e)),
    }
}
KeyCode::Char(']') => {
    let (gain, result) = {
        let m = self.state.lock().unwrap();
        let new_gain = (m.vga_gain + 2).min(62);
        let result = self.device.set_vga_gain(new_gain);
        (new_gain, result)
    };
    let mut m = self.state.lock().unwrap();
    match result {
        Ok(()) => {
            m.vga_gain = gain;
            m.push_log(format!("VGA gain → {} dB", gain));
        }
        Err(e) => m.push_log(format!("VGA gain error: {}", e)),
    }
}
```

`cargo build` must pass before moving on.

---

## Step 4 — AMP toggle (`a`)

**File:** `src/app.rs`

Add in the `Normal` mode key handler:

```rust
KeyCode::Char('a') => {
    let (enabled, result) = {
        let m = self.state.lock().unwrap();
        let new_state = !m.amp_enabled;
        let result = self.device.set_amp_enable(new_state);
        (new_state, result)
    };
    let mut m = self.state.lock().unwrap();
    match result {
        Ok(()) => {
            m.amp_enabled = enabled;
            m.push_log(format!("AMP {}", if enabled { "ON" } else { "OFF" }));
        }
        Err(e) => m.push_log(format!("AMP error: {}", e)),
    }
}
```

`cargo build` must pass before moving on.

---

## Step 5 — Footer update for input mode

**File:** `src/ui/footer.rs`

The footer now reads `InputMode` from `SdrMetrics` to show context-sensitive hints.
Update the function signature and add mode-aware rendering:

```rust
use crate::state::{SdrMetrics, InputMode};

pub fn render(f: &mut Frame, area: Rect, m: &SdrMetrics) {
    let text = match m.input_mode {
        InputMode::Normal => format!(
            " [Q] Quit | [SPACE] RX | [↑↓] LNA | [[]]] VGA | [A] AMP | [F] Freq | [R] Reset | [?] Help "
        ),
        InputMode::FrequencyInput => format!(
            " Frequency (MHz): [{}▌] | [Enter] confirm | [Esc] cancel ",
            m.input_buf
        ),
    };
    let footer = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(footer, area);
}
```

**Update all call sites.** The `draw()` function in `src/ui/mod.rs` currently calls
`footer::render(f, chunks.footer)`. Change it to:

```rust
footer::render(f, chunks.footer, m);
```

`cargo build` must pass before moving on.

---

## Step 6 — Frequency input mode — entry and typing (`f`)

**File:** `src/app.rs`

The event loop now needs to branch on `input_mode`. Restructure the key handler
to check mode first:

```rust
AppEvent::Key(key) => {
    let input_mode = self.state.lock().unwrap().input_mode.clone();
    match input_mode {
        InputMode::Normal => match key.code {
            KeyCode::Char('q') => return Ok(()),
            KeyCode::Char(' ') => {
                let mut m = self.state.lock().unwrap();
                m.rx_enabled = !m.rx_enabled;
            }
            KeyCode::Char('r') => {
                // reset_to_defaults now also calls hardware setters — see Step 8
                self.state.lock().unwrap().reset_to_defaults();
            }
            KeyCode::Char('f') => {
                let mut m = self.state.lock().unwrap();
                m.input_mode = InputMode::FrequencyInput;
                m.input_buf.clear();
                m.push_log("Enter frequency in MHz, then press Enter");
            }
            KeyCode::Char('?') => {
                self.show_help = !self.show_help;
            }
            // ... LNA, VGA, AMP keys from Steps 2–4 go here ...
            _ => {}
        },
        InputMode::FrequencyInput => match key.code {
            KeyCode::Esc => {
                let mut m = self.state.lock().unwrap();
                m.input_mode = InputMode::Normal;
                m.input_buf.clear();
                m.push_log("Frequency input cancelled");
            }
            KeyCode::Backspace => {
                self.state.lock().unwrap().input_buf.pop();
            }
            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' => {
                self.state.lock().unwrap().input_buf.push(c);
            }
            KeyCode::Enter => {
                // handled in Step 7
            }
            _ => {}
        },
    }
}
```

`cargo build` must pass before moving on.

---

## Step 7 — Frequency input mode — Enter (parse + hardware call)

**File:** `src/app.rs`

Fill in the `KeyCode::Enter` arm inside `FrequencyInput` mode:

```rust
KeyCode::Enter => {
    let (freq_hz, result) = {
        let m = self.state.lock().unwrap();
        let parsed = m.input_buf.parse::<f64>();
        match parsed {
            Ok(mhz) if mhz > 0.0 => {
                let hz = (mhz * 1_000_000.0) as u64;
                let result = self.device.set_frequency(hz);
                (Some(hz), Some(result))
            }
            _ => (None, None),
        }
    };
    let mut m = self.state.lock().unwrap();
    match (freq_hz, result) {
        (Some(hz), Some(Ok(()))) => {
            m.frequency = hz;
            m.input_mode = InputMode::Normal;
            m.input_buf.clear();
            m.push_log(format!("Frequency set to {:.3} MHz", hz as f64 / 1_000_000.0));
        }
        (Some(_), Some(Err(e))) => {
            m.push_log(format!("Frequency error: {}", e));
            // stay in FrequencyInput so user can correct
        }
        _ => {
            m.push_log(format!("Invalid frequency: '{}'", m.input_buf));
            // stay in FrequencyInput
        }
    }
}
```

`cargo build` must pass before moving on.

---

## Step 8 — Reset key wires hardware setters (`r`)

**File:** `src/state.rs` and `src/app.rs`

Currently `reset_to_defaults()` only updates the struct — it does not call hardware.
In Phase 5 the `r` key must also push the defaults back to the device.

Remove the `r` handler from the Normal match arm and replace with a method call
that also calls hardware. In `src/app.rs`, replace the `'r'` arm with:

```rust
KeyCode::Char('r') => {
    let results = [
        self.device.set_lna_gain(crate::state::DEFAULT_LNA_GAIN),
        self.device.set_vga_gain(crate::state::DEFAULT_VGA_GAIN),
        self.device.set_frequency(crate::state::DEFAULT_FREQUENCY),
        self.device.set_sample_rate(crate::state::DEFAULT_SAMPLE_RATE),
        self.device.set_amp_enable(false),
    ];
    let mut m = self.state.lock().unwrap();
    m.reset_to_defaults();
    for r in results {
        if let Err(e) = r {
            m.push_log(format!("Reset error: {}", e));
        }
    }
}
```

`cargo build` must pass before moving on.

---

## Step 9 — Help overlay (`?`)

**File:** `src/ui/overlay.rs`

This file already exists as a stub. Implement it:

```rust
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_help(f: &mut Frame) {
    // Center a 50×18 box in the terminal
    let area = centered_rect(50, 18, f.size());

    let text = "\
 [Q]       Quit\n\
 [SPACE]   Start / Stop RX\n\
 [↑] [↓]   LNA gain  +8 / −8 dB  (0–40 dB)\n\
 [[] []]   VGA gain  −2 / +2 dB  (0–62 dB)\n\
 [A]       Toggle AMP\n\
 [F]       Enter frequency (MHz)\n\
 [R]       Reset all to defaults\n\
 [?]       Toggle this help\n\
\n\
 In frequency input mode:\n\
   digits / .    type value\n\
   Backspace     delete last char\n\
   Enter         confirm\n\
   Esc           cancel\
";

    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title(" Help — press [?] to close ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Left),
        area,
    );
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}
```

**Wire into `src/ui/mod.rs`:** after `footer::render`, conditionally call the overlay:

```rust
pub fn draw(f: &mut Frame, m: &SdrMetrics, board_name: &str, fw: &str, serial: &str, show_help: bool) {
    let chunks = layout::build(f.size());
    header::render(f, chunks.header, board_name, fw, serial);
    telemetry::render(f, chunks.body_left, m, board_name, serial);
    gains::render(f, chunks.body_right, m);
    log::render(f, chunks.log, m);
    footer::render(f, chunks.footer, m);
    if show_help {
        overlay::render_help(f);
    }
}
```

Update the `terminal.draw` call in `App::run()`:

```rust
terminal.draw(|f| {
    ui::draw(f, &m, &self.board_name, &self.fw_version, &self.serial, self.show_help)
})?;
```

`cargo build` must pass before moving on.

---

## Step 10 — Final validation

```bash
cargo build --release   # zero errors, zero warnings
cargo clippy -- -D warnings  # zero findings
```

Manual test checklist — exercise with a real HackRF connected:

- [ ] `↑` / `↓` — LNA gauge updates immediately; out-of-range clamped; hardware error appears in log
- [ ] `[` / `]` — VGA gauge updates immediately; out-of-range clamped
- [ ] `a` — AMP state toggles; telemetry panel reflects new state
- [ ] `f` — footer switches to frequency input mode; typed characters appear in footer
- [ ] `f` → type invalid text → `Enter` — error in log, stays in FrequencyInput
- [ ] `f` → type valid MHz → `Enter` — frequency updates; mode returns to Normal
- [ ] `f` → `Esc` — input discarded; mode returns to Normal; log shows cancellation
- [ ] `r` — all gains, frequency, AMP reset to defaults; hardware matches display
- [ ] `?` — help overlay appears centered; press again, disappears
- [ ] `q` — app exits cleanly from both Normal and FrequencyInput modes
