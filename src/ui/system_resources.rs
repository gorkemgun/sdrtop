use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Gauge, Paragraph, Sparkline},
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

pub struct SystemResourcesPanel;

impl Panel for SystemResourcesPanel {
    fn name(&self) -> &'static str { "system_resources" }
    fn min_size(&self) -> (u16, u16) { (30, 10) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let block = Block::default()
            .title(" System Resources ")
            .borders(Borders::ALL);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);

        let cpu = state.process_cpu_pct.clamp(0.0, 100.0);
        let cpu_color = if cpu > 80.0       { Color::Red    }
            else if cpu > 50.0              { Color::Yellow }
            else                            { Color::Cyan   };
        f.render_widget(
            Gauge::default()
                .label(format!("CPU  {:.1}%", cpu))
                .ratio(cpu as f64 / 100.0)
                .style(Style::default().fg(cpu_color)),
            rows[0],
        );

        let rss = state.process_rss_mb;
        let rss_ratio = (rss as f64 / 512.0).min(1.0);
        let rss_color = if rss_ratio > 0.8 { Color::Red } else { Color::Magenta };
        f.render_widget(
            Gauge::default()
                .label(format!("RAM  {} MB", rss))
                .ratio(rss_ratio)
                .style(Style::default().fg(rss_color)),
            rows[1],
        );

        let throughput_mb = state.current_throughput_bps as f64 / 1_000_000.0;
        f.render_widget(
            Paragraph::new(Span::raw(format!("USB  {:.2} MB/s", throughput_mb))),
            rows[2],
        );

        let sparkline_data: Vec<u64> = state.throughput_history.iter().cloned().collect();
        f.render_widget(
            Sparkline::default()
                .data(&sparkline_data)
                .style(Style::default().fg(Color::Green)),
            rows[3],
        );
    }
}
