use chrono::{DateTime, Utc};
use color_eyre::owo_colors::OwoColorize;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{buffer::Buffer, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Modifier, Style, Stylize}, text::{Line, Span}, widgets::{Block, Borders, List, ListItem, Paragraph, Row, Table, Tabs, Widget}, Frame};
use soundcloud::sobjects::CloudPlaylists;
use strum::IntoEnumIterator;

use crate::{config::get_temp_dl_dir, dlp, screen::AppScreen};

#[derive(Debug, Clone)]
pub struct MainScreen {
    selected_tab: i8,
    selected_row: i32,
    max_rows: i32,
    tab_titles: Vec<String>,
    pub soundcloud: Option<CloudPlaylists>
}

impl AppScreen for MainScreen {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Right => self.next_tab(),
            KeyCode::Left => self.previous_tab(),
            KeyCode::Up => self.previous_row(),
            KeyCode::Down => self.next_row(),
            KeyCode::F(6) => self.download_row(),
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
            .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .select(self.selected_tab as usize)
            .style(Style::default().fg(Color::White));

        frame.render_widget(tabs, chunks[0]);

       /*let main_content = Paragraph::new("Main content goes here!")
            .block(Block::default().borders(Borders::ALL).title("Main")); */ 
        frame.render_widget(self.render_tab(), chunks[1]);  // Render into second chunk

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
        MainScreen { selected_row: -1, max_rows: 0, soundcloud: None, selected_tab: 0, tab_titles: vec!["YouTube".to_string(), "SoundCloud".to_string(), "Local Playlists".to_string(), "Settings".to_string()] }
    }

    fn next_tab(&mut self) {
        self.selected_tab = std::cmp::min(self.selected_tab+1, (self.tab_titles.len()-1).try_into().unwrap())
    }

    fn previous_tab(&mut self) {
        self.selected_tab = std::cmp::max(0, self.selected_tab-1);
    }

    fn previous_row(&mut self) {
        self.selected_row = std::cmp::max(0, self.selected_row-1);
    }

    fn next_row(&mut self) {
        self.selected_row = std::cmp::min(self.selected_row + 1, self.max_rows);
    }

    fn download_row(&mut self) {
        match self.selected_tab {
            1 => {// SC
                let playlist_url = self.soundcloud.as_ref().unwrap().collection.get(self.selected_row as usize).unwrap().permalink_url.clone();
                dlp::download_from_soundcloud(&playlist_url, &get_temp_dl_dir());
            },
            _ => {}
        }
    }

    fn render_tab(&self) -> Table<'_> {
        let rows = match self.selected_tab {
            1 => { // SC
                let mut v = Vec::new();
                v.push(Row::new(vec!["Id", "Title", "Songs Count", "Date", "IS"]).style(Style::default().fg(Color::Gray)));
                if let Some(s) = &self.soundcloud {
                    for (i, playlist) in (&s.collection).iter().enumerate() {
                        let date: DateTime<Utc> = playlist.created_at.parse().unwrap();
                        let mut row = Row::new(
                            vec![
                                        playlist.id.to_string(), 
                                        playlist.title.clone(), 
                                        [playlist.track_count.to_string(), " songs".to_string()].concat(), 
                                        format!("{}", date.format("%Y-%m-%d %H:%M")),
                                        "NO".to_string()
                                    ]
                        );
                        if self.selected_row == i as i32 {
                            row = row.style(Style::default().bg(Color::Yellow));
                        }
                        v.push(row);
                    }
                }
                v
            }
            _ => Vec::new()
        };

        // Create the table
        Table::new(rows, &[
                Constraint::Length(3),   // ID column
                Constraint::Percentage(50), // Playlist name column
                Constraint::Percentage(20), // Song count column
                Constraint::Percentage(30),
                Constraint::Length(2)
            ])
            .block(Block::default().borders(Borders::ALL).title(" Playlists "))
            .style(Style::default().fg(Color::White))
    }
}