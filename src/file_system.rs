use crate::{screen::AppScreen, theme::Theme};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Color, Line, Style, Stylize};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

#[derive(Default)]
pub struct FileSystem {}

impl AppScreen for FileSystem {
    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) {}

    fn render(&self, frame: &mut ratatui::Frame, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main content area
                Constraint::Length(1), // Status bar
            ])
            .split(frame.area());

        self.render_main(frame, chunks[0]);

        // Render Status Bar
        let status_bar = Paragraph::new(Line::from(vec![
            "<F5> SAVE AS PLAYLIST".bold(),
            " | ".dark_gray(),
            "<F6> SAVE AS IS".bold(),
            " | ".dark_gray(),
            "<F8> SELECT".bold(),
            " | ".dark_gray(),
            "<F9> DESELECT".bold(),
            " | ".dark_gray(),
            "<Q> QUIT".bold(),
        ]))
        .centered();
        frame.render_widget(status_bar, chunks[1]); // Render into third chunk
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl FileSystem {
    fn render_main(&self, frame: &mut Frame, area: Rect) {
        let mut v = vec![Row::new(vec!["Name", "Type", "Size", "Modified"])
            .style(Style::default().fg(Color::Gray))];

        // move this out to make hdd not suffer
        let paths = std::fs::read_dir("~/Documents").unwrap();

        for path in paths {
            v.push();
        }

        let table = Table::new(
            v,
            &[
                Constraint::Percentage(50),
                Constraint::Length(5),
                Constraint::Percentage(20),
                Constraint::Percentage(30),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title(" Documents "))
        .style(Style::default().fg(Color::Black));

        frame.render_widget(table, area);
    }
}
