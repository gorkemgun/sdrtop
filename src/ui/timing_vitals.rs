//! `timing_vitals` — the right column of the `lab_timing` preset's redesign.
//!
//! The host-pipeline health view: sample drops, ADC saturation and CPU each as a
//! labelled mini-graph (60 s rolling), then the USB link and ring-buffer state as
//! captioned bar sections, closed by a one-line vitals verdict + uptime. It reads
//! the same `state.signal` / `state.system` / `state.iq` vitals the standalone
//! `hardware_health` panel does, re-grouped to the redesign's three captions, plus
//! a per-device USB-link utilisation derived from `caps.sample_rate_max_hz`.

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::micro_common::{buf_color, drop_color, sat_color};
use crate::ui::panel::Panel;

pub struct TimingVitalsPanel;

/// Binary MB/s ceiling of the USB link for this device: the byte rate at the
/// device's maximum sample rate (8-bit I/Q ⇒ 2 bytes per complex sample). Honest
/// per-device headroom reference rather than a magic constant.
fn link_ceiling_mbps(sample_rate_max_hz: f64) -> f64 {
    (sample_rate_max_hz * 2.0) / (1024.0 * 1024.0)
}

/// Overrun margin: how much ring-buffer headroom remains below the ceiling, from
/// the session peak fill. Clamped to a sane 0..=100.
fn overrun_margin_pct(peak_fill_pct: f64) -> f64 {
    (100.0 - peak_fill_pct).clamp(0.0, 100.0)
}

/// `HH:MM:SS` uptime from a whole-second count.
fn fmt_uptime(secs: u64) -> String {
    format!("{:02}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60)
}

fn threshold_color(value: f64, warn: f64, crit: f64, theme: &crate::Theme) -> Color {
    if value >= crit { theme.status_crit } else if value >= warn { theme.status_warn } else { theme.status_ok }
}

/// `▰▰▰▱▱` segment bar: `ratio` filled in `color`, the rest dim. One row, `width` cells.
fn seg_bar(ratio: f64, width: usize, color: Color, theme: &crate::Theme) -> [Span<'static>; 2] {
    let ratio  = ratio.clamp(0.0, 1.0);
    let filled = (ratio * width as f64).round() as usize;
    [
        Span::styled("\u{25B0}".repeat(filled), Style::default().fg(color)),
        Span::styled("\u{25B1}".repeat(width.saturating_sub(filled)), Style::default().fg(theme.border_dim)),
    ]
}

/// `SECTION                       right caption` — bold left, dim right-aligned.
fn section_header(left: &'static str, right: &'static str, iw: usize, theme: &crate::Theme) -> Line<'static> {
    let lw = left.chars().count();
    let rw = right.chars().count();
    let gap = iw.saturating_sub(lw + rw + 1).max(1);
    Line::from(vec![
        Span::raw(" "),
        Span::styled(left, Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD)),
        Span::raw(" ".repeat(gap)),
        Span::styled(right, Style::default().fg(theme.label)),
    ])
}

impl Panel for TimingVitalsPanel {
    fn name(&self) -> &'static str { "timing_vitals" }
    fn min_size(&self) -> (u16, u16) { (30, 18) }
    fn focus_key(&self) -> Option<char> { Some('v') }
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
        &[("R", "Reset drop counter"), ("C", "Clear history")]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = !state.radio.hw_streaming;
        let key_style = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
        let mut title = vec![
            Span::raw(" Hardware "),
            Span::styled("V", key_style),
            Span::raw("itals"),
        ];
        if stale { title.push(Span::styled(" [STALE]", Style::default().fg(theme.stale))); }
        title.push(Span::raw(" "));
        let border = if focused { theme.border_focused } else if stale { theme.stale } else { theme.border_default };
        let block = Block::default()
            .title(Line::from(title))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border));
        let inner = block.inner(area);
        f.render_widget(block, area);
        if inner.width == 0 || inner.height == 0 { return; }
        let iw = inner.width as usize;

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // 0  pipeline caption
                Constraint::Length(1), // 1  drops label
                Constraint::Length(2), // 2  drops graph
                Constraint::Length(1), // 3  sat label
                Constraint::Length(2), // 4  sat graph
                Constraint::Length(1), // 5  cpu label
                Constraint::Length(2), // 6  cpu graph
                Constraint::Length(1), // 7  USB LINK header
                Constraint::Length(1), // 8  usb errors
                Constraint::Length(1), // 9  bus throughput
                Constraint::Length(1), // 10 link util bar
                Constraint::Length(1), // 11 RING BUFFER header
                Constraint::Length(1), // 12 fill depth bar
                Constraint::Length(1), // 13 peak fill
                Constraint::Length(1), // 14 overrun margin
                Constraint::Min(0),    // 15 verdict (last line)
            ])
            .split(inner);

        let lbl = Style::default().fg(theme.label);
        let val = Style::default().fg(theme.value);

        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled("host pipeline health \u{00b7} 60 s rolling", lbl),
        ])), rows[0]);

        // ── Sample drops ────────────────────────────────────────────────────────
        let dcol = drop_color(state.signal.drops_per_sec, theme);
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "), Span::styled("Sample drops ", lbl),
            Span::styled(format!("{}/s", state.signal.drops_per_sec), Style::default().fg(dcol)),
            Span::styled(format!("   session {}", state.signal.total_drops_session), lbl),
        ])), rows[1]);
        let drop_data: Vec<u64> = state.signal.drop_history.iter().copied().collect();
        crate::ui::charts::draw_mini_graph(f, rows[2], &drop_data, dcol);

        // ── ADC saturation ──────────────────────────────────────────────────────
        let scol = sat_color(state.signal.adc_saturation_pct, theme);
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "), Span::styled("ADC saturation ", lbl),
            Span::styled(format!("{:.1} %", state.signal.adc_saturation_pct), Style::default().fg(scol)),
            Span::styled(format!("   peak {:.1}%", state.signal.adc_saturation_peak), lbl),
        ])), rows[3]);
        let sat_data: Vec<u64> = state.signal.saturation_history.iter().map(|v| (*v * 1000.0) as u64).collect();
        crate::ui::charts::draw_mini_graph(f, rows[4], &sat_data, scol);

        // ── CPU / RAM ───────────────────────────────────────────────────────────
        let cpu = state.system.process_cpu_pct as f64;
        let ccol = threshold_color(cpu, 50.0, 80.0, theme);
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "), Span::styled("CPU load ", lbl),
            Span::styled(format!("{:.1} %", cpu), Style::default().fg(ccol)),
            Span::styled(format!("   RAM {} MB", state.system.process_rss_mb), lbl),
        ])), rows[5]);
        let cpu_data: Vec<u64> = state.system.cpu_history.iter().copied().collect();
        crate::ui::charts::draw_mini_graph(f, rows[6], &cpu_data, ccol);

        // ── USB link ────────────────────────────────────────────────────────────
        f.render_widget(section_header("USB LINK", "bulk transfer", iw, theme), rows[7]);
        let usb_recent: u64 = state.signal.usb_error_history.iter().sum();
        let ucol = if usb_recent > 0 { theme.status_crit }
                   else if state.signal.usb_errors_session > 0 { theme.status_warn }
                   else { theme.status_ok };
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "), Span::styled("USB errors ", lbl),
            Span::styled(format!("{}", state.signal.usb_errors_session), Style::default().fg(ucol)),
            Span::styled(" (session)", lbl),
        ])), rows[8]);

        let mbps    = state.timing.throughput_mean_mbps;
        let ceiling = link_ceiling_mbps(state.caps.sample_rate_max_hz);
        let util    = if ceiling > 0.0 { (mbps / ceiling).clamp(0.0, 1.0) } else { 0.0 };
        if stale {
            f.render_widget(Paragraph::new(Line::from(vec![
                Span::raw(" "), Span::styled("Bus throughput ", lbl), Span::styled("---", Style::default().fg(theme.stale)),
            ])), rows[9]);
        } else {
            f.render_widget(Paragraph::new(Line::from(vec![
                Span::raw(" "), Span::styled("Bus throughput ", lbl),
                Span::styled(format!("{mbps:.1} MB/s"), val),
                Span::styled(format!(" of {ceiling:.1} max"), lbl),
            ])), rows[9]);
        }
        let subtle = Style::default().fg(theme.label);
        let util_label = "link util ";
        let bar_w = iw.saturating_sub(util_label.chars().count() + 6).max(4);
        let [uf, ue] = seg_bar(util, bar_w, theme.value, theme);
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "), Span::styled(util_label, subtle), uf, ue,
            Span::styled(format!(" {:.0}%", util * 100.0), val),
        ])), rows[10]);

        // ── Ring buffer ─────────────────────────────────────────────────────────
        f.render_widget(section_header("RING BUFFER", "overrun margin", iw, theme), rows[11]);
        let fill = state.iq.buf_fill_pct as f64;
        let fcol = buf_color(state.iq.buf_fill_pct, theme);
        let fill_label = "fill depth ";
        let fbar_w = iw.saturating_sub(fill_label.chars().count() + 6).max(4);
        let [ff, fe] = seg_bar(fill / 100.0, fbar_w, fcol, theme);
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "), Span::styled(fill_label, subtle), ff, fe,
            Span::styled(format!(" {:.0}%", if stale { 0.0 } else { fill }), if stale { Style::default().fg(theme.stale) } else { Style::default().fg(fcol) }),
        ])), rows[12]);

        let peak_fill = state.iq.buf_fill_history.iter().copied().max().unwrap_or(0) as f64 / 10.0;
        let (peak_tag, peak_col) = if peak_fill >= 100.0 { ("hit ceiling", theme.status_crit) } else { ("headroom ok", theme.status_ok) };
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "), Span::styled("Peak fill ", lbl),
            Span::styled(format!("{peak_fill:.0} %"), Style::default().fg(buf_color(peak_fill as f32, theme))),
            Span::styled(format!("   {peak_tag}"), Style::default().fg(peak_col)),
        ])), rows[13]);

        let margin = overrun_margin_pct(peak_fill);
        f.render_widget(Paragraph::new(Line::from(vec![
            Span::raw(" "), Span::styled("Overrun margin ", lbl),
            Span::styled(format!("{margin:.0}%"), Style::default().fg(threshold_color(100.0 - margin, 50.0, 80.0, theme))),
        ])), rows[14]);

        // ── Verdict + uptime ────────────────────────────────────────────────────
        let verdict = if stale {
            Line::from(vec![Span::raw(" "), Span::styled("\u{25cb} idle \u{2014} RX stopped", Style::default().fg(theme.stale))])
        } else {
            let (mark, text, col) = match state.timing.timing_quality.severity() {
                0 => ("\u{2713}", "all vitals nominal", theme.status_ok),
                1 | 2 => ("\u{26a0}", "pipeline under load", theme.status_warn),
                _ => ("\u{26a0}", "overrun logged", theme.status_crit),
            };
            let up = state.radio.rx_start_time.map(|t| fmt_uptime(t.elapsed().as_secs()));
            let mut spans = vec![
                Span::raw(" "),
                Span::styled(format!("{mark} {text}"), Style::default().fg(col).add_modifier(Modifier::BOLD)),
            ];
            if let Some(up) = up {
                let used = 1 + mark.chars().count() + 1 + text.chars().count();
                let tail = format!("uptime {up}");
                let gap = iw.saturating_sub(used + tail.chars().count()).max(1);
                spans.push(Span::raw(" ".repeat(gap)));
                spans.push(Span::styled(tail, Style::default().fg(theme.label)));
            }
            Line::from(spans)
        };
        f.render_widget(Paragraph::new(verdict), rows[15]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_ceiling_is_byte_rate_at_max_sr() {
        // 20 Msps HackRF → 40 MB(byte)/s → ~38.1 binary MB/s.
        let c = link_ceiling_mbps(20_000_000.0);
        assert!((c - 38.147).abs() < 0.05, "got {c}");
        assert_eq!(link_ceiling_mbps(0.0), 0.0);
    }

    #[test]
    fn overrun_margin_clamps() {
        assert_eq!(overrun_margin_pct(0.0), 100.0);
        assert_eq!(overrun_margin_pct(62.0), 38.0);
        assert_eq!(overrun_margin_pct(100.0), 0.0);
        // A peak above the ceiling cannot push the margin negative.
        assert_eq!(overrun_margin_pct(140.0), 0.0);
    }

    #[test]
    fn uptime_formats_hms() {
        assert_eq!(fmt_uptime(0), "00:00:00");
        assert_eq!(fmt_uptime(15_127), "04:12:07");
        assert_eq!(fmt_uptime(59), "00:00:59");
    }

    #[test]
    fn seg_bar_total_width_is_n() {
        let t = crate::theme::Theme::sdr();
        for r in [0.0, 0.5, 1.0, 2.0_f64] {
            let [a, b] = seg_bar(r, 10, t.status_ok, &t);
            assert_eq!(a.content.chars().count() + b.content.chars().count(), 10, "ratio {r}");
        }
    }
}
