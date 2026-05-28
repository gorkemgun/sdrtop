use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::state::SdrMetrics;
use super::panel::Panel;

pub struct ObserverPanel;

impl Panel for ObserverPanel {
    fn name(&self) -> &'static str { "observer" }
    fn min_size(&self) -> (u16, u16) { (40, 10) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let dash = "—";
        let device   = state.observer_device.as_deref().unwrap_or(dash);
        let serial   = state.observer_serial.as_deref().unwrap_or(dash);
        let usb      = state.observer_usb.as_deref().unwrap_or(dash);
        let connected = state.observer_connected.as_deref().unwrap_or(dash);

        let mut lines: Vec<Line> = vec![
            Line::from(vec![
                Span::styled(" Observer Mode", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(""),
            Line::from(format!("  {}", device)),
            Line::from(format!("  Serial: {}", serial)),
            Line::from(format!("  USB {}", usb)),
            Line::from(format!("  Connected: {}", connected)),
            Line::from(""),
        ];

        if let Some(owner) = &state.observer_owner {
            lines.push(Line::from(format!("  In use by: {}", owner)));
            if let Some(cmdline) = &state.observer_cmdline {
                let truncated = if cmdline.len() > (area.width as usize).saturating_sub(4) {
                    format!("  {}…", &cmdline.chars().take((area.width as usize).saturating_sub(5)).collect::<String>())
                } else {
                    format!("  {}", cmdline)
                };
                lines.push(Line::from(truncated));
            }
            let uptime = state.observer_owner_uptime.as_deref().unwrap_or(dash);
            lines.push(Line::from(format!(
                "  CPU: {:.1}%  ·  RAM: {} MB  ·  Running: {}",
                state.observer_owner_cpu_pct,
                state.observer_owner_ram_mb,
                uptime,
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "  Owner: unknown (different user or process ended)",
                Style::default().fg(Color::DarkGray),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  Hardware controls disabled.",
            Style::default().fg(Color::DarkGray),
        )));

        let para = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" Observer Mode ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow)),
            );
        f.render_widget(para, area);
    }
}
