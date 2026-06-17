//! Shared "schematic deck" chrome — one frame language for every panel.
//!
//! Square (Plain) borders with a tick-tab nameplate on the top rule:
//! `┌╴LABEL╶─────┐`. This reads as precision field-instrument hardware rather
//! than a soft rounded window — without touching the colour palette.
//!
//! Panels build their own title spans (focus-key highlight, live state tags)
//! and wrap the name with [`nameplate`]; static panels use [`title`] directly.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};

/// A panel frame in the schematic deck language: square corners, single rule.
pub fn deck_block<'a>(border_color: Color) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .border_style(Style::default().fg(border_color))
}

/// Wrap nameplate label spans with tick end-caps: `╴…╶`. The caller may append
/// live state tags after the returned spans before building the title `Line`.
pub fn nameplate<'a>(label_spans: Vec<Span<'a>>, tick_color: Color) -> Vec<Span<'a>> {
    let mut spans = Vec::with_capacity(label_spans.len() + 2);
    spans.push(Span::styled("╴", Style::default().fg(tick_color)));
    spans.extend(label_spans);
    spans.push(Span::styled("╶", Style::default().fg(tick_color)));
    spans
}

/// A single uppercase nameplate label span (no focus key) in `color`.
pub fn label<'a>(text: &str, color: Color) -> Span<'a> {
    Span::styled(
        text.to_uppercase(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

/// A complete nameplate title `Line` for a static label: `╴LABEL╶`.
pub fn title<'a>(text: &str, label_color: Color, tick_color: Color) -> Line<'a> {
    Line::from(nameplate(vec![label(text, label_color)], tick_color))
}
