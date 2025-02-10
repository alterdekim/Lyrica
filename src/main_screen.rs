use color_eyre::owo_colors::OwoColorize;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{buffer::Buffer, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style, Stylize}, text::{Line, Span}, widgets::{Block, Borders, Paragraph, Tabs, Widget}, Frame};
use soundcloud::sobjects::CloudPlaylists;
use strum::IntoEnumIterator;

use crate::screen::AppScreen;

#[derive(Debug, Clone)]
pub struct MainScreen {
    selected_tab: i8,
    tab_titles: Vec<String>,
    pub soundcloud: Option<CloudPlaylists>
}

impl AppScreen for MainScreen {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('l') | KeyCode::Right => self.next_tab(),
            KeyCode::Char('h') | KeyCode::Left => self.previous_tab(),
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Tabs
                Constraint::Min(0),     // Main content area
                Constraint::Length(1),  // Status bar
            ])
            .split(frame.area());
        
        let tabs = Tabs::new(
                self.tab_titles.iter().map(|t| Span::raw(t.clone())).collect::<Vec<Span>>(),
            )
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .select(self.selected_tab as usize)
            .style(Style::default().fg(Color::White));

        frame.render_widget(tabs, chunks[0]);

        let main_content = Paragraph::new("Main content goes here!")
            .block(Block::default().borders(Borders::ALL).title("Main"));
        frame.render_widget(main_content, chunks[1]);  // Render into second chunk

        // Render Status Bar
        let status_bar = Paragraph::new(
            Line::from(
                vec!["◄ ► to change tab".bold(), " | ".dark_gray(), "<F5> SAVE FS".bold(), " | ".dark_gray(), "<F6> DL".bold(), " | ".dark_gray(), "<F8> DEL".bold(), " | ".dark_gray(), "<Q> QUIT".bold()]
            )
        )
        .centered();
        frame.render_widget(status_bar, chunks[2]);  // Render into third chunk
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl MainScreen {
    pub fn new() -> Self {
        MainScreen { soundcloud: None, selected_tab: 0, tab_titles: vec!["YouTube".to_string(), "SoundCloud".to_string(), "Local Playlists".to_string(), "Settings".to_string()] }
    }

    fn next_tab(&mut self) {
        self.selected_tab = std::cmp::min(self.selected_tab+1, (self.tab_titles.len()-1).try_into().unwrap())
    }

    fn previous_tab(&mut self) {
        self.selected_tab = std::cmp::max(0, self.selected_tab-1);
    }
}