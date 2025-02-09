use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{buffer::Buffer, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style, Stylize}, text::{Line, Span}, widgets::{Block, Borders, Paragraph, Tabs, Widget}, Frame};
use soundcloud::sobjects::CloudPlaylists;
use strum::IntoEnumIterator;

use crate::screen::AppScreen;

#[derive(Debug, Clone)]
pub struct MainScreen {
    selected_tab: u8,
    tab_titles: Vec<String>,
    pub soundcloud: Option<CloudPlaylists>
}

impl AppScreen for MainScreen {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        /*match key_event.code {
            KeyCode::Char('l') | KeyCode::Right => self.next_tab(),
            KeyCode::Char('h') | KeyCode::Left => self.previous_tab(),
            _ => {}
        }*/
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
        let status_bar = Paragraph::new("Press 'q' to quit | Arrow keys to navigate")
            .style(Style::default().fg(Color::Cyan));
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

    pub fn render_title(area: Rect, buf: &mut Buffer) {
        "Lyrica".bold().render(area, buf);
    }
    
    pub fn render_footer(area: Rect, buf: &mut Buffer) {
        Line::raw("◄ ► to change tab | <Q> to quit")
            .centered()
            .render(area, buf);
    }
}