//! `timing_diagnostics` — the left column of the `lab_timing` preset's redesign.
//!
//! An airy four-zone read-out of host-side stream timing, built as a single Line
//! stack and collapsed to fit (`chrome::collapse_spacers`) like the other lab side
//! panels so it breathes and fills:
//!
//!   1. CALLBACK TIMING  — period vs expected, host drift, jitter rms, the
//!      per-callback deviation percentiles, and a 60 s jitter trend.
//!   2. DEADLINE BUDGET  — p95 / p99 / peak deviation drawn against the deadline
//!      budget marker, plus the late-callback count.
//!   3. SAMPLE RATE      — configured vs actual, SR drift, throughput mean / σ.
//!   4. Verdict          — the 4-level `TimingQuality` call + a plain-language line.
//!
//! Every scalar comes from `state.timing`, rebuilt each poll window by the rx task.

use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::micro_common::sparkline;
use crate::ui::panel::Panel;
use crate::ui::timing_panel::{fmt_us, ppm_span, quality_color};

pub struct TimingDiagnosticsPanel;

/// Inline trend sparkline width.
const SPARK_W: usize = 18;

/// `SECTION              right caption` — bold left, dim right-aligned. Both
/// captions are owned, so the right side can carry a live value (the budget).
fn section(left: &str, right: &str, iw: usize, theme: &crate::Theme) -> Line<'static> {
    let gap = iw.saturating_sub(left.chars().count() + right.chars().count() + 1).max(1);
    Line::from(vec![
        Span::raw(" "),
        Span::styled(left.to_string(), Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD)),
        Span::raw(" ".repeat(gap)),
        Span::styled(right.to_string(), Style::default().fg(theme.label)),
    ])
}

/// One deadline-budget track: a `▬` ruler `track_w` wide with the budget marker
/// `┊` fixed at 75% of the axis, and the bar filled to `value` in a colour graded
/// by how far past the budget it sits. Returns just the track spans.
fn budget_track(value: u64, budget: u64, track_w: usize, theme: &crate::Theme) -> Vec<Span<'static>> {
    const BUDGET_FRAC: f64 = 0.75; // where the ┊ marker sits along the axis
    let full_scale = (budget as f64 / BUDGET_FRAC).max(1.0);
    let vfill = ((value as f64 / full_scale).clamp(0.0, 1.0) * track_w as f64).round() as usize;
    let bmark = (BUDGET_FRAC * track_w as f64).round() as usize;
    let color = if value <= budget { theme.status_ok }
                else if value <= budget * 2 { theme.status_warn }
                else { theme.status_crit };
    (0..track_w).map(|x| {
        if x == bmark {
            Span::styled("\u{250A}".to_string(), Style::default().fg(theme.label))
        } else if x < vfill {
            Span::styled("\u{25AC}".to_string(), Style::default().fg(color))
        } else {
            Span::styled("\u{25AC}".to_string(), Style::default().fg(theme.border_dim))
        }
    }).collect()
}

/// Two-line plain-language verdict copy, keyed off the 4-level severity, with live
/// numbers folded in (worst deviation and its share of the budget).
fn verdict_copy(severity: u8, peak_us: u64, budget_us: u64) -> [String; 2] {
    let pct = if budget_us > 0 { peak_us * 100 / budget_us } else { 0 };
    match severity {
        0 => [
            "Every callback met its deadline.".into(),
            format!("Worst {} ({pct}% of budget).", fmt_us(peak_us)),
        ],
        1 | 2 => [
            "Real-time deadlines under pressure.".into(),
            format!("Worst {} ({pct}%), no drops yet.", fmt_us(peak_us)),
        ],
        _ => [
            "Overrun — block dropped, resynced.".into(),
            "Ring buffer hit its ceiling.".into(),
        ],
    }
}

impl Panel for TimingDiagnosticsPanel {
    fn name(&self) -> &'static str { "timing_diagnostics" }
    fn min_size(&self) -> (u16, u16) { (34, 18) }
    fn focus_key(&self) -> Option<char> { Some('t') }
    fn focus_bindings(&self) -> &'static [(&'static str, &'static str)] {
        &[("R", "Reset jitter peak"), ("C", "Clear history")]
    }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = !state.radio.hw_streaming;
        let key_style = Style::default().fg(theme.value_hi).add_modifier(Modifier::BOLD);
        let name_style = Style::default().fg(theme.label).add_modifier(Modifier::BOLD);
        let mut title = vec![
            Span::raw(" "),
            Span::styled("T", key_style),
            Span::styled("iming Diagnostics", name_style),
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

        let t   = &state.timing;
        let lbl = Style::default().fg(theme.label);
        let val = Style::default().fg(theme.value);
        let dim = Style::default().fg(theme.stale);
        let dash = || Span::styled("---".to_string(), dim);
        // Pad a label to a fixed column so values line up down the zone.
        let field = |name: &str| Span::styled(format!(" {name:<9}"), lbl);

        let mut lines: Vec<Line> = Vec::new();

        // ── CALLBACK TIMING ─────────────────────────────────────────────────────
        lines.push(section("CALLBACK TIMING", "RX stream", iw, theme));
        lines.push(Line::from(if stale || t.cb_period_us == 0 {
            vec![field("Period"), dash()]
        } else {
            vec![
                field("Period"), Span::styled(fmt_us(t.cb_period_us), val),
                Span::styled(format!("   exp {}", fmt_us(t.cb_period_expected)), dim),
            ]
        }));
        lines.push(Line::from(if stale || t.cb_period_us == 0 {
            vec![field("Host drift"), dash()]
        } else {
            vec![field("Host drift"), ppm_span(t.cb_period_delta_ppm, theme)]
        }));
        lines.push(Line::from(if stale {
            vec![field("Jitter"), dash()]
        } else {
            vec![field("Jitter"), Span::styled(format!("\u{00b1}{} \u{00b5}s rms", t.cb_jitter_us), val)]
        }));
        lines.push(Line::from(if stale {
            vec![field(""), dash()]
        } else {
            vec![
                field(""),
                Span::styled(
                    format!("p95 {}  p99 {}  peak {} \u{00b5}s", t.dev_p95_us, t.dev_p99_us, t.dev_peak_us),
                    val),
            ]
        }));
        let jhist: Vec<f64> = state.iq.jitter_history.iter().map(|&v| v as f64).collect();
        lines.push(Line::from(vec![
            field("trend"),
            Span::styled(if stale { String::new() } else { sparkline(&jhist, SPARK_W) }, val),
            Span::styled(if stale { String::new() } else { "  60 s".into() }, dim),
        ]));

        lines.push(Line::raw(""));

        // ── DEADLINE BUDGET ─────────────────────────────────────────────────────
        let budget = t.deadline_budget_us;
        lines.push(section("DEADLINE BUDGET", &format!("\u{250A} = \u{00b1}{} \u{00b5}s", budget), iw, theme));
        for (name, v) in [("p95", t.dev_p95_us), ("p99", t.dev_p99_us), ("peak", t.dev_peak_us)] {
            let value_str = format!(" {} \u{00b5}s", v);
            let track_w = iw.saturating_sub(2 + 4 + value_str.chars().count()).max(6);
            let mut spans = vec![Span::styled(format!(" {name:<4}"), lbl)];
            if stale {
                spans.push(dash());
            } else {
                spans.extend(budget_track(v, budget, track_w, theme));
                spans.push(Span::styled(value_str, val));
            }
            lines.push(Line::from(spans));
        }
        lines.push(Line::from(if stale {
            vec![field("late"), dash()]
        } else if t.late_callbacks == 0 {
            vec![field("late"), Span::styled("\u{2713} none over budget".to_string(), Style::default().fg(theme.status_ok))]
        } else {
            let col = if t.late_callbacks * 20 > t.late_window { theme.status_crit } else { theme.status_warn };
            vec![field("late"), Span::styled(format!("{} / {} over budget", t.late_callbacks, t.late_window), Style::default().fg(col))]
        }));

        lines.push(Line::raw(""));

        // ── SAMPLE RATE ─────────────────────────────────────────────────────────
        lines.push(section("SAMPLE RATE", "clock integrity", iw, theme));
        let cfg_msps = state.radio.config_sample_rate / 1_000_000.0;
        lines.push(Line::from(if stale || state.radio.actual_sample_rate == 0 {
            vec![field("Rate"), Span::styled(format!("{cfg_msps:.3} MHz"), val), Span::raw("  "), dash()]
        } else {
            let act = state.radio.actual_sample_rate as f64 / 1_000_000.0;
            vec![field("Rate"), Span::styled(format!("{cfg_msps:.3} \u{2192} {act:.3} MHz"), val)]
        }));
        lines.push(Line::from(if stale || state.radio.actual_sample_rate == 0 {
            vec![field("SR drift"), dash()]
        } else {
            vec![field("SR drift"), ppm_span(t.sr_delta_ppm, theme)]
        }));
        lines.push(Line::from(if stale {
            vec![field("Throughput"), dash()]
        } else {
            vec![
                field("Throughput"), Span::styled(format!("{:.1} MB/s", t.throughput_mean_mbps), val),
                Span::styled(format!("   \u{03c3} {:.2}", t.throughput_std_mbps), dim),
            ]
        }));
        let thist: Vec<f64> = state.radio.throughput_history.iter().map(|&v| v as f64).collect();
        lines.push(Line::from(vec![
            field("flow"),
            Span::styled(if stale { String::new() } else { sparkline(&thist, SPARK_W) }, val),
        ]));

        lines.push(Line::raw(""));

        // ── Verdict ─────────────────────────────────────────────────────────────
        if stale {
            lines.push(Line::from(vec![Span::raw(" "), Span::styled("\u{25cb} IDLE \u{2014} RX stopped", dim)]));
        } else {
            let q = t.timing_quality;
            let col = quality_color(q, theme);
            let mark = if q.severity() == 0 { "\u{2713}" } else { "\u{26a0}" };
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled(format!("{mark} {}", q.label()), Style::default().fg(col).add_modifier(Modifier::BOLD)),
            ]));
            for d in verdict_copy(q.severity(), t.dev_peak_us, budget) {
                lines.push(Line::from(vec![Span::raw(" "), Span::styled(d, lbl)]));
            }
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::raw(" "),
                Span::styled("[R]", key_style), Span::styled(" reset peak  ", lbl),
                Span::styled("[C]", key_style), Span::styled(" clear counters", lbl),
            ]));
        }

        crate::ui::chrome::collapse_spacers(&mut lines, inner.height as usize);
        f.render_widget(Paragraph::new(lines), inner);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn budget_track_width_and_marker() {
        let t = Theme::sdr();
        let spans = budget_track(88, 600, 20, &t);
        // Exactly track_w cells, and the budget marker appears once.
        assert_eq!(spans.len(), 20);
        let marks = spans.iter().filter(|s| s.content == "\u{250A}").count();
        assert_eq!(marks, 1, "exactly one ┊ budget marker");
    }

    #[test]
    fn budget_track_colors_by_overage() {
        let t = Theme::sdr();
        // Under budget → the filled run is status_ok.
        let under = budget_track(100, 600, 24, &t);
        assert!(under.iter().any(|s| s.style.fg == Some(t.status_ok)));
        // Far over budget → status_crit appears in the fill.
        let over = budget_track(6_300, 600, 24, &t);
        assert!(over.iter().any(|s| s.style.fg == Some(t.status_crit)));
    }

    #[test]
    fn verdict_copy_folds_in_numbers_and_state() {
        let ok = verdict_copy(0, 210, 603);
        assert!(ok[0].contains("met its deadline"));
        assert!(ok[1].contains("210 µs") && ok[1].contains('%'));
        let bad = verdict_copy(3, 6_300, 603);
        assert!(bad[0].to_lowercase().contains("overrun"));
    }
}
