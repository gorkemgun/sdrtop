//! `command_rail` — the Command Rail layout's left instrument strip (`[1]`).
//!
//! A single vertical column that gathers what a poweruser reads at a glance:
//! the frequency hero (big segmented VFO + band tag), a value-first SIGNAL zone
//! (SNR with its short-term trend arrow, PWR, NF, SAT), the GAIN chain, the
//! STREAM health, and a one-line log foot. The header thins to status + dial
//! (see `SlimHeaderPanel`) and the frequency lives here instead.
//!
//! This is LÉPÉS 1 (the skeleton): live values, the existing SNR trend arrow, no
//! per-metric sparklines / recall slots / HUNT·MONITOR·BENCH modes yet — those
//! are later steps. Rendering is two non-overlapping `Paragraph`s (the stack and
//! the bottom-anchored log foot), so it never flickers.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::state::SdrMetrics;
use super::header::{active_digit_idx, gain_bar, vfo_spans, vfo_string};
use super::micro_common::{fft_stale, fmt_rbw, snr_color};
use super::panel::Panel;
use super::{bigdigits, chrome, log};
use crate::ui::band_plan::band_at;

pub struct CommandRailPanel;

/// Combined front-end gain for the TOTAL readout: primary + secondary stage when
/// the device has two (HackRF LNA+VGA), else just the primary (RTL-SDR tuner).
fn total_gain(lna: u32, vga: u32, has_second_stage: bool) -> u32 {
    if has_second_stage { lna + vga } else { lna }
}

/// Throughput as a compact `5.2 MB/s` string; `—` when not streaming.
fn fmt_mb(bps: u64) -> String {
    if bps == 0 { "—".to_string() } else { format!("{:.1} MB/s", bps as f64 / 1_000_000.0) }
}

/// Width of the gain bar given the rail's inner width — leaves room for the
/// `LNA ` label, a space, and a 2-col value. Clamped so it neither vanishes on a
/// narrow rail nor sprawls on a wide one.
fn gain_bar_width(inner_w: usize) -> usize {
    inner_w.saturating_sub(10).clamp(4, 12)
}

/// Trend arrow span for the SNR short-term delta: rising green / falling yellow /
/// steady dim. `None` until there is enough history (mirrors the micro view).
fn snr_arrow(delta: Option<f32>, theme: &crate::Theme) -> Option<Span<'static>> {
    let d = delta?;
    let (text, color): (&str, Color) = if d > 0.3 {
        ("↑", theme.status_ok)
    } else if d < -0.3 {
        ("↓", theme.status_warn)
    } else {
        ("→", theme.stale)
    };
    Some(Span::styled(text, Style::default().fg(color)))
}

/// Colour for the ADC-saturation value: calm below 10 %, warn to 50 %, crit above.
fn sat_color(pct: f32, theme: &crate::Theme) -> Color {
    if pct >= 50.0 { theme.status_crit }
    else if pct >= 10.0 { theme.status_warn }
    else { theme.value }
}

/// The frequency hero: the big 3-row block readout, or a single bold line when
/// the rail is too narrow for the block font. The actively-tuned digit is lit in
/// `value_hi` (the same digit the small VFO underlines), the rest in `value`, the
/// decimal point dim — all dim in observer mode.
fn freq_hero_lines(freq: u64, step: u64, observer: bool, inner_w: usize,
                   theme: &crate::Theme) -> Vec<Line<'static>> {
    let s = vfo_string(freq);

    // Narrow fallback: the existing single-line segmented VFO (+" MHz"). The +6
    // budget leaves room for the leading space and the " MHz" suffix.
    if bigdigits::big_width(&s) + 6 > inner_w {
        let col = if observer { theme.label } else { theme.value_hi };
        let mut spans = vec![Span::raw(" ")];
        spans.extend(vfo_spans(freq, step, col, theme.label, theme.value_hi));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("MHz", Style::default().fg(theme.label)));
        return vec![Line::from(spans)];
    }

    let active = active_digit_idx(freq, step);
    let chars: Vec<char> = s.chars().collect();
    let mut rows: [Vec<Span<'static>>; 3] =
        [vec![Span::raw(" ")], vec![Span::raw(" ")], vec![Span::raw(" ")]];
    for (i, &c) in chars.iter().enumerate() {
        let color = if observer { theme.label }
            else if Some(i) == active { theme.value_hi }
            else if c == '.' { theme.label }
            else { theme.value };
        let g = bigdigits::glyph(c);
        for (r, row) in rows.iter_mut().enumerate() {
            if i > 0 { row.push(Span::raw(" ")); }
            row.push(Span::styled(g[r].to_string(), Style::default().fg(color)));
        }
    }
    // "MHz" rides the middle row, just past the digits.
    rows[1].push(Span::raw(" "));
    rows[1].push(Span::styled("MHz", Style::default().fg(theme.label)));
    let [r0, r1, r2] = rows;
    vec![Line::from(r0), Line::from(r1), Line::from(r2)]
}

/// `[FM]  SR 2.0M · RBW 1.5 kHz` — the band chip plus sample-rate / resolution
/// context, sitting just under the frequency hero.
fn band_sr_line(state: &SdrMetrics, theme: &crate::Theme) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    if let Some(b) = band_at(state.radio.frequency) {
        spans.push(Span::styled(format!(" {b} "), Style::default()
            .fg(Color::Rgb(4, 6, 15)).bg(theme.value_hi).add_modifier(Modifier::BOLD)));
        spans.push(Span::raw("  "));
    }
    let sr = state.radio.config_sample_rate / 1_000_000.0;
    spans.push(Span::styled(format!("SR {sr:.1}M"), Style::default().fg(theme.label)));
    spans.push(Span::styled(" · ", Style::default().fg(theme.border_dim)));
    let rbw = match state.waterfall.last_fft.as_ref().filter(|fr| fr.enbw_hz > 0.0) {
        Some(fr) => fmt_rbw(fr.enbw_hz),
        None     => "—".to_string(),
    };
    spans.push(Span::styled(format!("RBW {rbw}"), Style::default().fg(theme.label)));
    Line::from(spans)
}

impl Panel for CommandRailPanel {
    fn name(&self) -> &'static str { "command_rail" }
    fn min_size(&self) -> (u16, u16) { (22, 12) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let border = if focused { theme.border_focused } else { theme.border_dim };
        let block = chrome::deck_block(border)
            .title(chrome::title("Command", theme.label, border));
        let inner = block.inner(area);
        f.render_widget(block, area);
        chrome::corner_accents(f, area, border);
        if inner.width == 0 || inner.height == 0 { return; }

        let iw = inner.width as usize;
        let stale = fft_stale(state);
        let observer = state.observer.active;
        let active = state.radio.hw_streaming && !observer;

        let lbl   = |s: &str| Span::styled(format!("{s:<5}"), Style::default().fg(theme.label));
        let dash  = || Span::styled("—".to_string(), Style::default().fg(theme.stale));
        // Dim `╴SECTION╶` divider, matching the deck nameplate language.
        let section = |name: &str| Line::from(chrome::nameplate(
            vec![chrome::label(name, theme.label)], theme.border_dim));

        let mut lines: Vec<Line> = Vec::new();

        // --- FREQ HERO ---------------------------------------------------------
        lines.extend(freq_hero_lines(state.radio.frequency, state.spectrum.step_hz,
                                     observer, iw, theme));
        lines.push(band_sr_line(state, theme));
        lines.push(Line::raw(""));

        // --- SIGNAL ------------------------------------------------------------
        lines.push(section("Signal"));
        // SNR + trend arrow.
        let snr = state.signal.peak_to_nf_db;
        let mut snr_row = vec![Span::raw(" "), lbl("SNR")];
        if stale {
            snr_row.push(dash());
        } else {
            snr_row.push(Span::styled(format!("{:.1} dB", snr),
                Style::default().fg(snr_color(snr, theme))));
            if let Some(a) = snr_arrow(state.signal.snr_delta(), theme) {
                snr_row.push(Span::raw("  "));
                snr_row.push(a);
            }
        }
        lines.push(Line::from(snr_row));
        // PWR.
        let pwr = state.signal.channel_power_dbfs;
        let pwr_span = if stale || !pwr.is_finite() { dash() }
            else { Span::styled(format!("{:.1} dBFS", pwr), Style::default().fg(theme.value)) };
        lines.push(Line::from(vec![Span::raw(" "), lbl("PWR"), pwr_span]));
        // NF (from the latest FFT frame).
        let nf_span = match state.waterfall.last_fft.as_ref().filter(|_| !stale) {
            Some(fr) => Span::styled(format!("{:.1} dBFS", fr.noise_floor), Style::default().fg(theme.value)),
            None     => dash(),
        };
        lines.push(Line::from(vec![Span::raw(" "), lbl("NF"), nf_span]));
        // SAT.
        let sat = state.signal.adc_saturation_pct;
        let sat_span = if !active { dash() }
            else { Span::styled(format!("{:.1} %", sat), Style::default().fg(sat_color(sat, theme))) };
        lines.push(Line::from(vec![Span::raw(" "), lbl("SAT"), sat_span]));
        lines.push(Line::raw(""));

        // --- GAIN --------------------------------------------------------------
        lines.push(section("Gain"));
        let gm = &state.caps.gain;
        let bw = gain_bar_width(iw);
        let val_col = if active { theme.value } else { theme.label };
        // Front-end boost (AMP / AGC).
        let (boost_val, boost_col) = if observer { ("—".to_string(), theme.label) }
            else if state.radio.amp_enabled { ("ON".to_string(), theme.value_hi) }
            else { ("OFF".to_string(), theme.label) };
        lines.push(Line::from(vec![
            Span::raw(" "), lbl(gm.boost_label()),
            Span::styled(boost_val, Style::default().fg(boost_col)),
        ]));
        // Primary stage (LNA / Tuner).
        let (p_f, p_e) = gain_bar(state.radio.lna_gain, gm.primary_max_db(), bw);
        lines.push(Line::from(vec![
            Span::raw(" "), lbl(gm.primary_label()),
            Span::styled(p_f, Style::default().fg(if active { theme.status_ok } else { theme.label })),
            Span::styled(p_e, Style::default().fg(theme.border_dim)),
            Span::raw(" "),
            Span::styled(format!("{:2}", state.radio.lna_gain), Style::default().fg(val_col)),
        ]));
        // Secondary stage (HackRF VGA only).
        if gm.has_second_stage() {
            let (v_f, v_e) = gain_bar(state.radio.vga_gain, 62, bw);
            lines.push(Line::from(vec![
                Span::raw(" "), lbl("VGA"),
                Span::styled(v_f, Style::default().fg(if active { theme.status_warn } else { theme.label })),
                Span::styled(v_e, Style::default().fg(theme.border_dim)),
                Span::raw(" "),
                Span::styled(format!("{:2}", state.radio.vga_gain), Style::default().fg(val_col)),
            ]));
        }
        let total = total_gain(state.radio.lna_gain, state.radio.vga_gain, gm.has_second_stage());
        lines.push(Line::from(vec![
            Span::raw(" "), lbl("TOTAL"),
            Span::styled(format!("{total} dB"), Style::default().fg(theme.value)),
        ]));
        lines.push(Line::raw(""));

        // --- STREAM ------------------------------------------------------------
        lines.push(section("Stream"));
        let stream_val = |s: String| Span::styled(s, Style::default().fg(if active { theme.value } else { theme.label }));
        lines.push(Line::from(vec![Span::raw(" "), lbl("DROP"),
            stream_val(format!("{} /s", state.signal.drops_per_sec))]));
        lines.push(Line::from(vec![Span::raw(" "), lbl("BUF"),
            stream_val(format!("{:.0} %", state.iq.buf_fill_pct))]));
        lines.push(Line::from(vec![Span::raw(" "), lbl("USB"),
            stream_val(fmt_mb(if active { state.radio.current_throughput_bps } else { 0 }))]));

        // Split off the bottom inner row for the log foot so the stack and the
        // foot never overlap (no flicker), and the foot stays anchored.
        let (stack_area, foot_area) = if inner.height >= 4 {
            (Rect { height: inner.height - 1, ..inner },
             Some(Rect { x: inner.x, y: inner.y + inner.height - 1, width: inner.width, height: 1 }))
        } else {
            (inner, None)
        };
        f.render_widget(Paragraph::new(lines), stack_area);

        if let Some(foot) = foot_area {
            if let Some(e) = state.ui.log.back() {
                let foot_line = Line::from(vec![
                    Span::raw(" "),
                    log::lamp(e.level, theme),
                    Span::raw(" "),
                    Span::styled(log::fmt_clock(e.at_epoch_secs), Style::default().fg(theme.border_dim)),
                    Span::raw(" "),
                    Span::styled(e.text.as_ref(), Style::default().fg(theme.value)),
                ]);
                f.render_widget(Paragraph::new(foot_line), foot);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn total_gain_sums_only_with_second_stage() {
        assert_eq!(total_gain(32, 30, true), 62);  // HackRF LNA+VGA
        assert_eq!(total_gain(40, 99, false), 40); // RTL-SDR tuner only
    }

    #[test]
    fn fmt_mb_blanks_when_idle() {
        assert_eq!(fmt_mb(0), "—");
        assert_eq!(fmt_mb(5_200_000), "5.2 MB/s");
    }

    #[test]
    fn gain_bar_width_clamps() {
        assert_eq!(gain_bar_width(10), 4);   // tiny rail → floor
        assert_eq!(gain_bar_width(0), 4);
        assert_eq!(gain_bar_width(22), 12);  // wide rail → ceiling
        assert_eq!(gain_bar_width(18), 8);   // mid → 18-10
    }

    #[test]
    fn snr_arrow_directions() {
        let t = Theme::sdr();
        assert!(snr_arrow(None, &t).is_none());
        assert_eq!(snr_arrow(Some(1.0), &t).unwrap().style.fg, Some(t.status_ok));
        assert_eq!(snr_arrow(Some(-1.0), &t).unwrap().style.fg, Some(t.status_warn));
        assert_eq!(snr_arrow(Some(0.0), &t).unwrap().style.fg, Some(t.stale));
    }

    #[test]
    fn sat_color_escalates() {
        let t = Theme::sdr();
        assert_eq!(sat_color(0.0, &t), t.value);
        assert_eq!(sat_color(20.0, &t), t.status_warn);
        assert_eq!(sat_color(80.0, &t), t.status_crit);
    }
}
