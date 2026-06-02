use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::hardware::Device;
use crate::state::SdrMetrics;
use crate::ui::panel::Panel;
use crate::ui::rf_calc::{adc_utilisation_ratio, estimate_mds_dbm, estimate_nf_db, gain_advice};

pub struct RfChainPanel;

fn fmt_hz(hz: u32) -> String {
    if hz == 0        { return "---".to_string(); }
    if hz >= 1_000_000 {
        format!("{:.3} MHz", hz as f64 / 1_000_000.0)
    } else {
        format!("{} kHz", hz / 1_000)
    }
}

/// Format a length in cm into the most readable unit.
fn fmt_cm(cm: f64) -> String {
    if cm >= 100.0      { format!("{:.2} m",  cm / 100.0) }
    else if cm >= 1.0   { format!("{:.1} cm", cm) }
    else                { format!("{:.1} mm", cm * 10.0) }
}

/// λ and λ/4 from frequency in Hz.  Returns "--- / ---" if frequency is 0.
fn fmt_wavelength(freq_hz: u64) -> String {
    if freq_hz == 0 { return "--- / ---".to_string(); }
    let lambda    = 3e10 / freq_hz as f64;   // cm  (c = 3×10¹⁰ cm/s)
    let lambda_4  = lambda / 4.0;
    format!("{} / {}", fmt_cm(lambda), fmt_cm(lambda_4))
}

impl Panel for RfChainPanel {
    fn name(&self) -> &'static str { "rf_chain" }
    fn min_size(&self) -> (u16, u16) { (32, 16) }

    fn render(&self, f: &mut Frame, area: ratatui::layout::Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let stale = !state.radio.hw_streaming;
        let title = if stale { " RF Chain [STALE] " } else { " RF Chain " };
        let border_color = if focused { theme.border_focused }
            else if stale { theme.stale }
            else { theme.border_default };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let bb_bw = state.radio.bb_filter_hz;
        let total_gain = state.radio.lna_gain as i32
            + state.radio.vga_gain as i32
            + if state.radio.amp_enabled { 14 } else { 0 };

        let lbl  = Style::default().fg(theme.label);
        let val  = Style::default().fg(theme.value);
        let hi   = Style::default().fg(theme.value_hi);

        let (advice_text, advice_sev) = if stale {
            ("--- (RX not streaming)", 0u8)
        } else {
            gain_advice(&state.iq.iq_amplitude_hist)
        };
        let advice_color = if stale { theme.stale } else {
            match advice_sev {
                2 => theme.status_crit,
                1 => theme.status_warn,
                _ => theme.status_ok,
            }
        };

        // ADC utilisation gauge: fraction of samples in mid-range bins (8–23).
        // Show as stale (zero bar, stale color) when RX is not streaming.
        let (util_ratio, util_color) = if stale {
            (0.0, theme.stale)
        } else {
            let ratio = adc_utilisation_ratio(&state.iq.iq_amplitude_hist);
            let color = if ratio > 0.5      { theme.status_ok }
                        else if ratio > 0.2 { theme.status_warn }
                        else                { theme.status_crit };
            (ratio, color)
        };

        // Estimated cascade Noise Figure (Friis)
        let nf_db = estimate_nf_db(state.radio.amp_enabled, state.radio.lna_gain);
        let nf_color = if nf_db < 4.0      { theme.status_ok }
                       else if nf_db < 8.0 { theme.status_warn }
                       else                { theme.status_crit };

        // Frequency, wavelength, and sample rate strings
        let freq_str  = format!("{:.3} MHz", state.radio.frequency as f64 / 1_000_000.0);
        let wl_str    = fmt_wavelength(state.radio.frequency);
        let sr_str    = format!("{:.3} MHz", state.radio.config_sample_rate / 1_000_000.0);

        // Gain chain: AMP[14] → LNA[xx] → VGA[xx] = total dB
        let chain_line = if state.radio.amp_enabled {
            Line::from(vec![
                Span::styled("AMP", lbl),
                Span::styled(format!("[{}]", 14), hi),
                Span::styled(" → ", lbl),
                Span::styled("LNA", lbl),
                Span::styled(format!("[{}]", state.radio.lna_gain), hi),
                Span::styled(" → ", lbl),
                Span::styled("VGA", lbl),
                Span::styled(format!("[{}]", state.radio.vga_gain), hi),
                Span::styled(format!(" = {} dB", total_gain), Style::default().fg(theme.value_hi)),
            ])
        } else {
            Line::from(vec![
                Span::styled("LNA", lbl),
                Span::styled(format!("[{}]", state.radio.lna_gain), hi),
                Span::styled(" → ", lbl),
                Span::styled("VGA", lbl),
                Span::styled(format!("[{}]", state.radio.vga_gain), hi),
                Span::styled(format!(" = {} dB", total_gain), Style::default().fg(theme.value_hi)),
            ])
        };

        let info_rows: &[Line] = &[
            Line::from(vec![
                Span::styled(format!("{:<13}", "Freq"),       lbl),
                Span::styled(freq_str, hi),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "λ / λ/4"),   lbl),
                Span::styled(wl_str, val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "Sample rate"), lbl),
                Span::styled(sr_str, val),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "BB filter"),  lbl),
                Span::styled(fmt_hz(bb_bw), val),
            ]),
            Line::from(vec![Span::raw("")]),
            chain_line,
            Line::from(vec![
                Span::styled(format!("{:<13}", "Est. NF"),  lbl),
                Span::styled(format!("~{:.1} dB", nf_db),  Style::default().fg(nf_color)),
                Span::styled("  (Friis)", Style::default().fg(theme.border_dim)),
            ]),
            {
                let (mds_str, mds_color) = match estimate_mds_dbm(bb_bw, nf_db) {
                    Some(mds) => {
                        let color = if mds < -95.0      { theme.status_ok }
                                    else if mds < -85.0 { theme.status_warn }
                                    else                { theme.status_crit };
                        (format!("~{:.0} dBm", mds), color)
                    }
                    None => ("---".to_string(), theme.stale),
                };
                Line::from(vec![
                    Span::styled(format!("{:<13}", "MDS"),   lbl),
                    Span::styled(mds_str, Style::default().fg(mds_color)),
                ])
            },
            Line::from(vec![Span::raw("")]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "Board"),   lbl),
                Span::styled(Device::board_rev_name(state.system.board_rev), Style::default().fg(theme.border_dim)),
            ]),
            Line::from(vec![
                Span::styled(format!("{:<13}", "USB API"), lbl),
                Span::styled(format!("{:#06x}", state.system.usb_api_version), Style::default().fg(theme.border_dim)),
            ]),
            Line::from(vec![Span::raw("")]),
            Line::from(vec![
                Span::styled(advice_text, Style::default().fg(advice_color)),
            ]),
        ];

        // Reserve 1 row at the bottom for the ADC utilisation ▐ bar
        let n_info = info_rows.len().min(inner.height.saturating_sub(1) as usize);
        if inner.height < 2 { return; }

        let sections = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(n_info as u16),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner);

        let row_constraints: Vec<Constraint> = (0..n_info).map(|_| Constraint::Length(1)).collect();
        let row_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(row_constraints)
            .split(sections[0]);
        for (i, line) in info_rows.iter().take(n_info).enumerate() {
            f.render_widget(Paragraph::new(line.clone()), row_areas[i]);
        }

        crate::ui::charts::draw_hbar(
            f, sections[2], util_ratio,
            "ADC util ",
            &format!("{:.0}%", util_ratio * 100.0),
            util_color, theme,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wavelength_2400mhz() {
        // λ = 3e10 / 2.4e9 = 12.5 cm,  λ/4 = 3.1 cm
        let s = fmt_wavelength(2_400_000_000);
        assert!(s.contains("12.5 cm"), "got: {}", s);
        assert!(s.contains("3.1 cm"),  "got: {}", s);
    }

    #[test]
    fn wavelength_433mhz() {
        // λ = 3e10 / 4.33e8 ≈ 69.3 cm,  λ/4 ≈ 17.3 cm
        let s = fmt_wavelength(433_000_000);
        assert!(s.contains("69."), "got: {}", s);
        assert!(s.contains("17."), "got: {}", s);
    }

    #[test]
    fn wavelength_zero_returns_dashes() {
        assert_eq!(fmt_wavelength(0), "--- / ---");
    }

}
