use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Points},
        Block, Borders, Paragraph,
    },
    Frame,
};

use crate::state::SdrMetrics;
use crate::ui::panel::Panel;

const DB_MIN: f32 = -120.0;
const DB_MAX: f32 = 0.0;

pub struct SpectrumPanel;

impl Panel for SpectrumPanel {
    fn name(&self) -> &'static str { "spectrum" }
    fn min_size(&self) -> (u16, u16) { (40, 10) }

    fn render(&self, f: &mut Frame, area: Rect, state: &SdrMetrics) {
        let stale = state.last_fft_frame.as_ref().map(|fr| {
            fr.timestamp.elapsed() > std::time::Duration::from_millis(500)
        }).unwrap_or(false);

        let title = if stale { " Spectrum [STALE] " } else { " Spectrum " };

        match state.last_fft_frame.as_ref() {
            None => {
                f.render_widget(
                    Paragraph::new("Waiting for RX\u{2026}")
                        .block(Block::default().title(title).borders(Borders::ALL))
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(Color::DarkGray)),
                    area,
                );
            }
            Some(frame) => {
                // Split: left 6 cols = dBFS labels, right = canvas + freq axis
                let cols = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Length(6), Constraint::Min(1)])
                    .split(area);

                let rows = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(4), Constraint::Length(1)])
                    .split(cols[1]);

                let canvas_area = rows[0];
                let freq_area   = rows[1];
                let db_area     = cols[0];

                let n = frame.bins_dbfs.len() as f64;
                let title_style = if stale {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };

                // Spectrum canvas
                let bins = frame.bins_dbfs.clone();
                let peaks = frame.peak_hold.clone();
                let noise_floor = frame.noise_floor;
                f.render_widget(
                    Canvas::default()
                        .block(
                            Block::default()
                                .title(Span::styled(title, title_style))
                                .borders(Borders::TOP | Borders::RIGHT | Borders::BOTTOM),
                        )
                        .x_bounds([0.0, n])
                        .y_bounds([DB_MIN as f64, DB_MAX as f64])
                        .paint(move |ctx| {
                            // Spectrum bars
                            for (i, &db) in bins.iter().enumerate() {
                                let y = db.clamp(DB_MIN, DB_MAX) as f64;
                                ctx.draw(&CanvasLine {
                                    x1: i as f64, y1: DB_MIN as f64,
                                    x2: i as f64, y2: y,
                                    color: Color::Green,
                                });
                            }
                            // Peak hold as individual points
                            for (i, &db) in peaks.iter().enumerate() {
                                let y = db.clamp(DB_MIN, DB_MAX) as f64;
                                ctx.draw(&Points {
                                    coords: &[(i as f64, y)],
                                    color: Color::Yellow,
                                });
                            }
                            // Noise floor as a horizontal line
                            let nf = noise_floor.clamp(DB_MIN, DB_MAX) as f64;
                            ctx.draw(&CanvasLine {
                                x1: 0.0, y1: nf,
                                x2: n,   y2: nf,
                                color: Color::DarkGray,
                            });
                        }),
                    canvas_area,
                );

                // Frequency axis labels (1 row below canvas)
                let bw = frame.sample_rate;
                let left_hz = frame.center_freq_hz as f64 - bw / 2.0;
                let freq_labels: Vec<String> = (0..=4)
                    .map(|i| format!("{:.2}M", (left_hz + bw * i as f64 / 4.0) / 1_000_000.0))
                    .collect();
                f.render_widget(
                    Paragraph::new(Span::raw(format!(
                        "{:<12}{:<12}{:<12}{:<12}{}",
                        freq_labels[0], freq_labels[1],
                        freq_labels[2], freq_labels[3], freq_labels[4]
                    )))
                    .style(Style::default().fg(Color::DarkGray)),
                    freq_area,
                );

                // dBFS labels (left column, 5 levels top to bottom)
                let db_text: String = (0..=4)
                    .map(|i| {
                        let db = DB_MAX - (DB_MAX - DB_MIN) * i as f32 / 4.0;
                        format!("{:+4.0}\n", db)
                    })
                    .collect();
                f.render_widget(
                    Paragraph::new(db_text)
                        .block(Block::default().borders(Borders::TOP | Borders::LEFT | Borders::BOTTOM))
                        .style(Style::default().fg(Color::DarkGray)),
                    db_area,
                );
            }
        }
    }
}
