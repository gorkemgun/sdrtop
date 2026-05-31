use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct HardwareHealthPanel;

fn threshold_color(value: f64, warn: f64, crit: f64, theme: &crate::Theme) -> Color {
    if value >= crit      { theme.status_crit }
    else if value >= warn { theme.status_warn }
    else                  { theme.status_ok   }
}

impl Panel for HardwareHealthPanel {
    fn name(&self) -> &'static str { "hardware_health" }
    fn min_size(&self) -> (u16, u16) { (30, 12) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics, theme: &crate::Theme, focused: bool) {
        let border_color = if focused { theme.border_focused } else { theme.border_default };
        let block = Block::default()
            .title(" Hardware Health ")
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Min(0),
            ])
            .split(inner);

        let drop_color = threshold_color(state.signal.drops_per_sec as f64, 1.0, 10.0, theme);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "Drops: {}/s  (session total: {})",
                    state.signal.drops_per_sec, state.signal.total_drops_session
                ),
                Style::default().fg(drop_color),
            )),
            rows[0],
        );
        let drop_data: Vec<u64> = state.signal.drop_history.iter().cloned().collect();
        crate::ui::charts::draw_mini_graph(f, rows[1], &drop_data, drop_color);

        let sat_color = threshold_color(state.signal.adc_saturation_pct as f64, 1.0, 5.0, theme);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "ADC sat: {:.1}%  (peak: {:.1}%)",
                    state.signal.adc_saturation_pct, state.signal.adc_saturation_peak
                ),
                Style::default().fg(sat_color),
            )),
            rows[2],
        );
        let sat_data: Vec<u64> = state.signal.saturation_history.iter()
            .map(|v| (*v * 1000.0) as u64)  // millipercent — preserves sub-1% values on graph
            .collect();
        crate::ui::charts::draw_mini_graph(f, rows[3], &sat_data, sat_color);

        let period_ms = state.iq.cb_period_us as f64 / 1000.0;
        let jitter_color = threshold_color(state.iq.cb_jitter_us as f64, 200.0, 1000.0, theme);
        f.render_widget(
            Paragraph::new(Span::styled(
                format!(
                    "CB period {:.1} ms  jitter ±{} µs",
                    period_ms, state.iq.cb_jitter_us
                ),
                Style::default().fg(jitter_color),
            )),
            rows[4],
        );
        let jitter_data: Vec<u64> = state.iq.jitter_history.iter().cloned().collect();
        crate::ui::charts::draw_mini_graph(f, rows[5], &jitter_data, jitter_color);

        let usb_color = if state.signal.usb_errors_session > 0 { theme.status_crit } else { theme.status_ok };
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("USB errors: {} (session)", state.signal.usb_errors_session),
                Style::default().fg(usb_color),
            )),
            rows[6],
        );
        let usb_err_data: Vec<u64> = state.signal.usb_error_history.iter().cloned().collect();
        crate::ui::charts::draw_mini_graph(f, rows[7], &usb_err_data, usb_color);
    }
}
