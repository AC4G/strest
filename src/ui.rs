use std::error::Error;
use crossterm::{execute, terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{backend::CrosstermBackend, layout::{Constraint, Direction, Layout}, prelude::{text, Backend}, style::{Color, Style}, text::Span, widgets::{Block, Borders, Paragraph, Wrap}, Terminal};
use std::time::Duration;
use std::io;
use tokio::sync::{broadcast::{self}, watch};

use crate::args::TesterArgs;
    
pub trait UiActions {
    fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>, Box<dyn Error>>;
    fn cleanup();
    fn render<B: Backend>(
        terminal: &mut Terminal<B>,
        elapsed_time: &Duration,
        current_request: &u64,
        successful_requests: &u64,
        target_duration: &u64,
        latencies: &Vec<(f64, f64)>,
        rps: &f64,
        rpm: &f64
    );
}

pub struct Ui;
    
impl UiActions for Ui {
    fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>, Box<dyn Error>> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Terminal::new(CrosstermBackend::new(io::stdout()))?)
    }

    fn cleanup() {
        disable_raw_mode().ok();
        execute!(std::io::stdout(), LeaveAlternateScreen).ok();
    }

    fn render<B: Backend>(
    terminal: &mut Terminal<B>,
    elapsed_time: &Duration,
    current_request: &u64,
    successful_requests: &u64,
    target_duration: &u64,
    latencies: &Vec<(f64, f64)>,
    rps: &f64,
    rpm: &f64
) {
    terminal.draw(|f| {
        let size = f.size();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Min(10),
            ])
            .split(size);

        let stats_text = Paragraph::new(vec![
            text::Line::from(vec![
                Span::from("Elapsed Time: "),
                Span::styled(format!("{:.2}s", elapsed_time.as_secs_f64()), Style::default().fg(Color::Green)),
                Span::from("   Target: "),
                Span::styled(format!("{target_duration}s"), Style::default().fg(Color::Yellow)),
            ]),
            text::Line::from(vec![
                Span::from("Requests: "),
                Span::styled(current_request.to_string(), Style::default().fg(Color::LightBlue)),
                Span::from("   Success: "),
                Span::styled(successful_requests.to_string(), Style::default().fg(Color::Magenta)),
            ]),
            text::Line::from(vec![
                Span::from("RPS: "),
                Span::styled(format!("{}", rps), Style::default().fg(Color::Cyan)),
                Span::from("   RPM: "),
                Span::styled(format!("{}", rpm), Style::default().fg(Color::Cyan)),
            ]),
        ])
        .block(Block::default().title("Stats").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

        f.render_widget(stats_text, chunks[0]);

        let mut lat_values: Vec<f64> = latencies.iter().map(|(_, latency)| *latency).collect();
        lat_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p50 = percentile(&lat_values, 0.50);
        let p90 = percentile(&lat_values, 0.90);
        let p99 = percentile(&lat_values, 0.99);

        let percentiles_text = Paragraph::new(vec![text::Line::from(vec![
            Span::from("P50: "),
            Span::styled(format!("{:.2}ms", p50), Style::default().fg(Color::Green)),
            Span::from("   P90: "),
            Span::styled(format!("{:.2}ms", p90), Style::default().fg(Color::Yellow)),
            Span::from("   P99: "),
            Span::styled(format!("{:.2}ms", p99), Style::default().fg(Color::Red)),
        ])])
        .block(Block::default().title("Latency Percentiles").borders(Borders::ALL))
        .wrap(Wrap { trim: true });

        f.render_widget(percentiles_text, chunks[1]);

        // Chart logic
        let data_points: Vec<(f64, f64)> = latencies.clone();
        let y_max = lat_values.iter().cloned().fold(0.0, f64::max).ceil().max(10.0);
        let x_max = data_points.last().map(|(x, _)| *x).unwrap_or(0.0).ceil();
        let x_min = if x_max > 10.0 { x_max - 10.0 } else { 0.0 };

        let datasets = vec![ratatui::widgets::Dataset::default()
            .name("Latency Chart")
            .marker(ratatui::symbols::Marker::Dot)
            .style(Style::default().fg(Color::Cyan))
            .data(&data_points)];

        let chart = ratatui::widgets::Chart::new(datasets)
            .block(Block::default().borders(Borders::ALL))
            .x_axis(
                ratatui::widgets::Axis::default()
                    .title("Window Second")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([x_min, x_max])
                    .labels(vec![
                        Span::raw(format!("{:.0}", x_min)),
                        Span::raw(format!("{:.0}", (x_min + x_max) / 2.0)),
                        Span::raw(format!("{:.0}", x_max)),
                    ]),
            )
            .y_axis(
                ratatui::widgets::Axis::default()
                    .title("Latency (ms)")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, y_max])
                    .labels(vec![
                        Span::raw("0"),
                        Span::raw(format!("{:.0}", y_max / 2.0)),
                        Span::raw(format!("{:.0}", y_max)),
                    ]),
            );

        f.render_widget(chart, chunks[2]);
    })
    .unwrap();
}
}

fn percentile(data: &[f64], percentile: f64) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let rank = percentile * (data.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        data[lower]
    } else {
        let weight = rank - lower as f64;
        data[lower] * (1.0 - weight) + data[upper] * weight
    }
}

#[derive(Debug, Clone)]
pub struct UiData {
    pub elapsed_time: Duration,
    pub current_requests: u64,
    pub successful_requests: u64,
    pub latencies: Vec<(f64, f64)>,
    pub rps: f64,
    pub rpm: f64,
}

impl UiData {
    pub fn new(
        elapsed_time: Duration,
        current_requests: u64,
        successful_requests: u64,
        latencies: Vec<(f64, f64)>,
        rps: f64,
        rpm: f64,
    ) -> Self {
        Self {
            elapsed_time,
            current_requests,
            successful_requests,
            latencies,
            rps,
            rpm,
        }
    }

    pub fn default() -> Self {
        Self {
            elapsed_time: Duration::from_secs(0),
            current_requests: 0,
            successful_requests: 0,
            latencies: Vec::new(),
            rps: 0.0,
            rpm: 0.0,
        }
    }
}

pub fn setup_render_ui(
    args: &TesterArgs,
    shutdown_tx: &broadcast::Sender<u16>,
    ui_tx: &watch::Sender<UiData>,
) -> tokio::task::JoinHandle<()> {
    let mut ui_rx = ui_tx.subscribe();
    let mut shutdown_rx = shutdown_tx.subscribe();
    let target_duration = args.target_duration;

    tokio::spawn(async move {
        let mut terminal = Ui::setup_terminal().unwrap();

        loop {
            tokio::select! {
                Ok(_) = shutdown_rx.recv() => {
                    Ui::cleanup();
                    break;
                }
                Ok(_) = ui_rx.changed() => {
                    let msg = ui_rx.borrow().clone();
                    Ui::render(
                        &mut terminal,
                        &msg.elapsed_time,
                        &msg.current_requests,
                        &msg.successful_requests,
                        &target_duration,
                        &msg.latencies,
                        &msg.rps,
                        &msg.rpm
                    );
                }
            }
        }
    })
}
