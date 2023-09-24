use std::error::Error;
use crossterm::{execute, terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen}};
use ratatui::{Terminal, backend::CrosstermBackend, layout::{Layout, Constraint, Direction}, text::Span, style::{Style, Color}, widgets::{Block, Borders, Paragraph, Padding}, prelude::{Backend, text}};
use std::time::Duration;
use std::io;
    
pub trait UiActions {
    fn setup_terminal() -> Result<Terminal<CrosstermBackend<std::io::Stdout>>, Box<dyn Error>>;
    fn cleanup();
    fn render_ui<B: Backend>(
        terminal: &mut Terminal<B>,
        elapsed_time: &Duration,
        estimated_duration: &Duration
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

    fn render_ui<B: Backend>(
        terminal: &mut Terminal<B>,
        elapsed_time: &Duration,
        estimated_duration: &Duration
    ) {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([Constraint::Percentage(20), Constraint::Percentage(30), Constraint::Percentage(50)].as_ref())
                .split(f.size());

            let time_box = Block::default()
                .title("Time and Duration")
                .borders(Borders::ALL)
                .padding(Padding { left: (2), right: (0), top: (1), bottom: (1) });

            let elapsed_time_str = format!("{:.2}", elapsed_time.as_secs_f64());
            let estimated_duration_str = format!("{:.2}", estimated_duration.as_secs_f64());

            let elapsed_time_text = text::Line::from(vec![
                Span::from("Elapsed Time: "),
                Span::styled(elapsed_time_str, Style::default().fg(Color::Green)),
                Span::from("s")
            ]);
            let duration_time_text = text::Line::from(vec![
                Span::from("Estimated Duration: "),
                Span::styled(estimated_duration_str, Style::default().fg(Color::Cyan)),
                Span::from("s")
            ]);

            let time_text = Paragraph::new(vec![
                elapsed_time_text,
                duration_time_text
            ]).style(Style::default().bg(Color::Black));

            let time_box_inner = time_box.inner(chunks[0]);
            f.render_widget(time_box, chunks[0]);
            f.render_widget(time_text, time_box_inner);
        })
        .unwrap();
    }
}

pub enum UiData {
    ElapsedAndEstimatedTime(Duration, Duration),
    Terminate
}
