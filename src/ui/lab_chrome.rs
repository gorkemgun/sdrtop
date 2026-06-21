//! Lab "instrument mode" chrome — the two thin bars that wrap every measurement
//! lab (`[5]`–`[9]`):
//!
//! - [`LabBannerPanel`] (top): `LAB · RF CHAIN [6]   REF —   AVG OFF   CAL —   MKR 2        ▶ LIVE`
//! - [`LabMarkerPanel`] (bottom): `MKR1 92.800 MHz -19.1 dBFS   MKR2 …   Δ …        [hints]`
//!
//! Both are borderless single-line bars (text row + a dim hairline) laid into the
//! lab presets' Top/Bottom slots like the footer — no engine changes. They read
//! the measurement flags from [`LabState`](crate::state::LabState) and the marker
//! list from `SpectrumState` (not duplicated). Every field is width-aware:
//! lower-priority fields drop out whole rather than clip mid-word.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::state::{SdrMetrics, SpectrumMarker};
use super::panel::Panel;

/// Map an active preset name to its lab banner label and the number key that
/// selects it (the *current* key map, which we keep — see the implementation
/// plan). `None` for any non-lab preset, so the chrome bars no-op if they ever
/// render outside a lab.
pub fn lab_label(preset: &str) -> Option<(&'static str, char)> {
    match preset {
        "lab_iq"     => Some(("I/Q QUALITY", '5')),
        "lab_rf"     => Some(("RF CHAIN",    '6')),
        "lab_timing" => Some(("HOST TIMING", '7')),
        "lab_signal" => Some(("SIGNAL",      '8')),
        "lab_sweep"  => Some(("SWEEP",       '9')),
        _ => None,
    }
}

/// Precise marker-readout frequency: `92.800 MHz` / `433.920 MHz` / `1.234500 GHz`.
fn fmt_freq_mhz(hz: u64) -> String {
    if hz >= 1_000_000_000 {
        format!("{:.6} GHz", hz as f64 / 1e9)
    } else {
        format!("{:.3} MHz", hz as f64 / 1e6)
    }
}

/// `Δ` readout between two markers: frequency span + (optional) level difference,
/// e.g. `5.400 MHz 12.3 dB` or just `5.400 MHz` when a level is unavailable.
fn fmt_delta(df_hz: u64, dl_db: Option<f32>) -> String {
    let f = if df_hz >= 1_000_000_000 {
        format!("{:.6} GHz", df_hz as f64 / 1e9)
    } else {
        format!("{:.3} MHz", df_hz as f64 / 1e6)
    };
    match dl_db {
        Some(d) => format!("{f} {:.1} dB", d.abs()),
        None    => f,
    }
}

/// dBFS level at `freq_hz`, read from the latest FFT frame's bins. `None` if there
/// is no frame yet or the frequency is outside the captured span.
fn level_at_freq(state: &SdrMetrics, freq_hz: u64) -> Option<f32> {
    let fr = state.waterfall.last_fft.as_ref()?;
    let n = fr.bins_dbfs.len();
    if n == 0 { return None; }
    let left = fr.center_freq_hz as f64 - fr.sample_rate / 2.0;
    let frac = (freq_hz as f64 - left) / fr.sample_rate;
    if !(0.0..=1.0).contains(&frac) { return None; }
    let idx = (frac * (n - 1) as f64).round() as usize;
    fr.bins_dbfs.get(idx.min(n - 1)).copied()
}

/// Display width (columns) of a span run — every glyph we use here is single-width.
fn span_w(spans: &[Span]) -> usize {
    spans.iter().map(|s| s.content.chars().count()).sum()
}

/// A full-width dim hairline rule.
fn hairline(iw: usize, theme: &crate::Theme) -> Line<'static> {
    Line::from(Span::styled("\u{2500}".repeat(iw), Style::default().fg(theme.border_dim)))
}

// ── Banner (top bar) ────────────────────────────────────────────────────────

fn banner_lines(state: &SdrMetrics, theme: &crate::Theme, iw: usize) -> Vec<Line<'static>> {
    let (label, num) = match lab_label(&state.ui.active_preset) {
        Some(x) => x,
        None    => return vec![Line::raw("")],
    };
    let dim  = Style::default().fg(theme.label);
    let bold = Style::default().fg(theme.label).add_modifier(Modifier::BOLD);
    let hi   = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
    let val  = Style::default().fg(theme.value);

    // Left zone: " LAB · RF CHAIN [6]"
    let left: Vec<Span> = vec![
        Span::raw(" "),
        Span::styled("LAB", bold),
        Span::styled(" \u{00B7} ", dim),
        Span::styled(label, hi),
        Span::styled(" [", dim),
        Span::styled(num.to_string(), hi),
        Span::styled("]", dim),
    ];
    let lw = span_w(&left);

    // Right zone: live / freeze.
    let streaming = state.radio.hw_streaming && !state.observer.active;
    let mut right: Vec<Span> = if streaming {
        vec![Span::styled("\u{25B6} ", Style::default().fg(theme.status_ok)),
             Span::styled("LIVE ", Style::default().fg(theme.status_ok).add_modifier(Modifier::BOLD))]
    } else {
        vec![Span::styled("\u{2016} ", Style::default().fg(theme.status_warn)),
             Span::styled("FRZ ", Style::default().fg(theme.status_warn).add_modifier(Modifier::BOLD))]
    };
    let mut rw = span_w(&right);
    if iw <= lw + rw + 1 { right.clear(); rw = 0; } // too narrow for the right zone

    // Middle fields in priority order (REF > MKR > AVG > CAL).
    let mkr = state.spectrum.markers.len();
    let fields = [
        ("REF", state.lab.ref_label()),
        ("MKR", mkr.to_string()),
        ("AVG", state.lab.avg_label()),
        ("CAL", state.lab.cal_label().to_string()),
    ];
    let mut mid: Vec<Span> = Vec::new();
    let mut mw = 0usize;
    for (lbl, value) in fields {
        let cand = vec![
            Span::raw("   "),
            Span::styled(lbl, dim),
            Span::raw(" "),
            Span::styled(value, val),
        ];
        let cw = span_w(&cand);
        if lw + mw + cw + rw + 1 <= iw { mid.extend(cand); mw += cw; }
    }

    let filler = iw.saturating_sub(lw + mw + rw).max(1);
    let mut spans = left;
    spans.extend(mid);
    spans.push(Span::raw(" ".repeat(filler)));
    spans.extend(right);

    vec![Line::from(spans), hairline(iw, theme)]
}

// ── Marker bar (bottom bar) ─────────────────────────────────────────────────

fn marker_spans(idx: usize, mk: Option<&SpectrumMarker>, state: &SdrMetrics,
                theme: &crate::Theme) -> Vec<Span<'static>> {
    let dim  = Style::default().fg(theme.label);
    let val  = Style::default().fg(theme.value);
    match mk {
        Some(m) => {
            let lvl = level_at_freq(state, m.freq_hz)
                .map(|d| format!("{d:.1} dBFS"))
                .unwrap_or_else(|| "\u{2014}".to_string());
            vec![
                Span::styled(format!("MKR{idx} "), dim),
                Span::styled(fmt_freq_mhz(m.freq_hz), val),
                Span::raw(" "),
                Span::styled(lvl, val),
            ]
        }
        None => vec![Span::styled(format!("MKR{idx} "), dim),
                     Span::styled("\u{2014}", Style::default().fg(theme.border_dim))],
    }
}

fn marker_lines(state: &SdrMetrics, theme: &crate::Theme, iw: usize) -> Vec<Line<'static>> {
    let dim = Style::default().fg(theme.label);
    let key = Style::default().fg(theme.value_hi);

    let m1 = state.spectrum.markers.first();
    let m2 = state.spectrum.markers.get(1);

    // MKR1 always shown; the rest fill as room allows.
    let mut spans: Vec<Span> = vec![Span::raw(" ")];
    spans.extend(marker_spans(1, m1, state, theme));
    let mut used = span_w(&spans);

    let try_add = |cand: Vec<Span<'static>>, used: &mut usize, spans: &mut Vec<Span<'static>>| {
        let cw = span_w(&cand);
        if *used + cw + 1 <= iw { spans.extend(cand); *used += cw; }
    };

    if m2.is_some() {
        let mut c = vec![Span::raw("   ")];
        c.extend(marker_spans(2, m2, state, theme));
        try_add(c, &mut used, &mut spans);
    }

    // Δ between the two markers.
    if let (Some(a), Some(b)) = (m1, m2) {
        let df = (b.freq_hz as i64 - a.freq_hz as i64).unsigned_abs();
        let dl = match (level_at_freq(state, a.freq_hz), level_at_freq(state, b.freq_hz)) {
            (Some(x), Some(y)) => Some(y - x),
            _ => None,
        };
        let c = vec![
            Span::raw("   "),
            Span::styled("\u{0394} ", dim),
            Span::styled(fmt_delta(df, dl), Style::default().fg(theme.value).add_modifier(Modifier::BOLD)),
        ];
        try_add(c, &mut used, &mut spans);
    }

    // Right-side focus hints from the currently focused panel, if any room.
    let hints: Vec<Span> = state.ui.focused_panel_bindings.iter()
        .flat_map(|(k, l)| vec![
            Span::styled(format!("[{k}] "), key),
            Span::styled(format!("{l}  "), dim),
        ]).collect();
    let hw = span_w(&hints);
    if !hints.is_empty() && used + hw + 2 <= iw {
        let filler = iw.saturating_sub(used + hw);
        spans.push(Span::raw(" ".repeat(filler)));
        spans.extend(hints);
    }

    vec![hairline(iw, theme), Line::from(spans)]
}

// ── Panels ──────────────────────────────────────────────────────────────────

/// Top instrument-state banner for the lab presets.
pub struct LabBannerPanel;

impl Panel for LabBannerPanel {
    fn name(&self) -> &'static str { "lab_banner" }
    fn min_size(&self) -> (u16, u16) { (20, 1) }
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        if area.width == 0 || area.height == 0 { return; }
        let lines = banner_lines(state, theme, area.width as usize);
        f.render_widget(Paragraph::new(lines), area);
    }
}

/// Bottom marker / delta readout bar for the lab presets.
pub struct LabMarkerPanel;

impl Panel for LabMarkerPanel {
    fn name(&self) -> &'static str { "lab_marker" }
    fn min_size(&self) -> (u16, u16) { (20, 1) }
    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, _focused: bool) {
        if area.width == 0 || area.height == 0 { return; }
        let lines = marker_lines(state, theme, area.width as usize);
        f.render_widget(Paragraph::new(lines), area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lab_label_maps_current_key_numbers() {
        assert_eq!(lab_label("lab_rf"),     Some(("RF CHAIN", '6')));
        assert_eq!(lab_label("lab_iq"),     Some(("I/Q QUALITY", '5')));
        assert_eq!(lab_label("lab_signal"), Some(("SIGNAL", '8')));
        assert_eq!(lab_label("command_rail"), None);
        assert_eq!(lab_label("spectrum"),     None);
    }

    #[test]
    fn fmt_freq_mhz_picks_unit() {
        assert_eq!(fmt_freq_mhz(92_800_000),    "92.800 MHz");
        assert_eq!(fmt_freq_mhz(433_920_000),   "433.920 MHz");
        assert_eq!(fmt_freq_mhz(1_234_500_000), "1.234500 GHz");
    }

    #[test]
    fn fmt_delta_formats_with_and_without_level() {
        assert_eq!(fmt_delta(5_400_000, Some(-12.3)), "5.400 MHz 12.3 dB");
        assert_eq!(fmt_delta(5_400_000, None),        "5.400 MHz");
    }
}
