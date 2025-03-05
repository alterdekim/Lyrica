use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use crate::{dlp::DownloadProgress, screen::AppScreen};

#[derive(Default)]
pub struct LoadingScreen {
    pub progress: Option<(u32, u32, ratatui::style::Color)>,
    pub artwork_progress: Option<(u32, u32)>,
    pub s_progress: Option<DownloadProgress>,
}

impl AppScreen for LoadingScreen {
    fn handle_key_event(&mut self, _key_event: crossterm::event::KeyEvent) {}

    fn render(&self, frame: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main content area
                Constraint::Length(1), // Status bar
            ])
            .split(frame.area());

        self.render_progress(frame, chunks[0]);

        // Render Status Bar
        let status_bar = Paragraph::new(Line::from(vec!["<Q> QUIT".bold()])).centered();
        frame.render_widget(status_bar, chunks[1]); // Render into third chunk
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl LoadingScreen {
    fn render_progress(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main content
                Constraint::Length(6), // Progress bar
                Constraint::Length(6), // Progress bar
            ])
            .split(area);

        let main_content = Paragraph::new("Please wait").block(
            Block::default()
                .borders(Borders::ALL)
                .title("Downloading has started!"),
        );

        frame.render_widget(main_content, chunks[0]);

        if self.progress.is_some() {
            self.render_overall(frame, chunks[1]);
        }

        if self.artwork_progress.is_some() {
            self.render_artwork_progress(frame, chunks[2]);
        } else if self.s_progress.is_some() {
            self.render_current(frame, chunks[2]);
        }
    }

    fn render_artwork_progress(&self, frame: &mut Frame, area: Rect) {
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Generating album covers "),
            )
            .gauge_style(Style::default().fg(Color::LightBlue))
            .ratio(
                self.artwork_progress.unwrap().0 as f64 / self.artwork_progress.unwrap().1 as f64,
            )
            .label("Generating album covers...");

        frame.render_widget(gauge, area);
    }

    fn render_current(&self, frame: &mut Frame, area: Rect) {
        let s: String = self
            .s_progress
            .as_ref()
            .unwrap()
            .progress_percentage
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let ratio: f64 = s.parse::<f64>().unwrap_or(0.0);

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title(format!(
                " Downloading Item (ETA: {}) ",
                self.s_progress.as_ref().unwrap().eta
            )))
            .gauge_style(Style::default().fg(Color::Green))
            .ratio(ratio / 100.0)
            .label(self.s_progress.as_ref().unwrap().progress_total.to_string());

        frame.render_widget(gauge, area);
    }

    fn render_overall(&self, frame: &mut Frame, area: Rect) {
        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Downloading Playlist "),
            )
            .gauge_style(Style::default().fg(self.progress.unwrap().2))
            .ratio(self.progress.unwrap().0 as f64 / self.progress.unwrap().1 as f64)
            .label(format!(
                "{:}/{:}",
                self.progress.unwrap().0,
                self.progress.unwrap().1
            ));

        frame.render_widget(gauge, area);
    }
}
