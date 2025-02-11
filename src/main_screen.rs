use chrono::{DateTime, Utc};
use color_eyre::owo_colors::OwoColorize;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Row, Table, Tabs},
    Frame,
};
use soundcloud::sobjects::CloudPlaylists;
use strum::IntoEnumIterator;
use tokio::sync::mpsc::UnboundedSender;

use crate::{screen::AppScreen, sync::AppEvent};

struct Playlist {
    id: u64,
    title: String,
    link: String,
    created_at: String,
    track_count: u32,
}

pub struct MainScreen {
    selected_tab: i8,
    selected_row: i32,
    max_rows: i32,
    tab_titles: Vec<String>,
    soundcloud: Option<Vec<Playlist>>,
    pub progress: Option<(u32, u32)>,
    sender: UnboundedSender<AppEvent>,
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
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Main content area
                Constraint::Length(1), // Status bar
            ])
            .split(frame.area());

        let tabs = Tabs::new(
            self.tab_titles
                .iter()
                .map(|t| Span::raw(t.clone()))
                .collect::<Vec<Span>>(),
        )
        .block(Block::default().borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .select(self.selected_tab as usize)
        .style(Style::default().fg(Color::White));

        frame.render_widget(tabs, chunks[0]);

        if self.selected_tab != -1 {
            //frame.render_widget(self.render_tab(), chunks[1]);
            self.render_tab(frame, chunks[1]);
        } else {
            self.render_progress(frame, chunks[1]);
        }

        // Render Status Bar
        let status_bar = Paragraph::new(Line::from(vec![
            "◄ ► to change tab".bold(),
            " | ".dark_gray(),
            "<F5> SAVE FS".bold(),
            " | ".dark_gray(),
            "<F6> DL".bold(),
            " | ".dark_gray(),
            "<F8> DEL".bold(),
            " | ".dark_gray(),
            "<Q> QUIT".bold(),
        ]))
        .centered();
        frame.render_widget(status_bar, chunks[2]); // Render into third chunk
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl MainScreen {
    pub fn new(sender: UnboundedSender<AppEvent>) -> Self {
        MainScreen {
            selected_row: -1,
            max_rows: 0,
            soundcloud: None,
            progress: None,
            selected_tab: 0,
            tab_titles: vec![
                "YouTube".to_string(),
                "SoundCloud".to_string(),
                "Local Playlists".to_string(),
                "Settings".to_string(),
            ],
            sender,
        }
    }

    pub fn download_screen(&mut self) {
        self.selected_tab = -1;
    }

    fn update_max_rows(&mut self) {
        self.max_rows = match self.selected_tab {
            1 => self.soundcloud.as_deref().unwrap_or(&[]).len(),
            _ => 0,
        }
        .try_into()
        .unwrap();
    }

    fn next_tab(&mut self) {
        self.selected_tab = std::cmp::min(
            self.selected_tab + 1,
            (self.tab_titles.len() - 1).try_into().unwrap(),
        );
        self.update_max_rows();
    }

    fn previous_tab(&mut self) {
        self.selected_tab = std::cmp::max(0, self.selected_tab - 1);
        self.update_max_rows();
    }

    fn previous_row(&mut self) {
        self.selected_row = std::cmp::max(0, self.selected_row - 1);
    }

    fn next_row(&mut self) {
        self.selected_row = std::cmp::min(self.selected_row + 1, self.max_rows - 1);
    }

    fn download_row(&mut self) {
        if self.selected_tab == 1 {
            // SC
            let playlist_url = self
                .soundcloud
                .as_ref()
                .unwrap()
                .get(self.selected_row as usize)
                .unwrap()
                .link
                .clone();
            let _ = self.sender.send(AppEvent::DownloadPlaylist(playlist_url));
        }
    }

    pub fn set_soundcloud_playlists(&mut self, pl: CloudPlaylists) {
        self.soundcloud = Some(
            pl.collection
                .iter()
                .map(|p| Playlist {
                    id: p.id,
                    created_at: p.created_at.clone(),
                    title: p.title.clone(),
                    link: p.permalink_url.clone(),
                    track_count: p.track_count,
                })
                .collect(),
        );
    }

    fn render_progress(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Progress bar
            ])
            .split(area);

        let main_content = Paragraph::new("Main content goes here!")
            .block(Block::default().borders(Borders::ALL).title("Main"));

        frame.render_widget(main_content, chunks[0]);

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Downloading Playlist "),
            )
            .gauge_style(Style::default().fg(Color::Green))
            .ratio(self.progress.unwrap().0 as f64 / self.progress.unwrap().1 as f64)
            .label(format!(
                "{:}/{:}",
                self.progress.unwrap().0,
                self.progress.unwrap().1
            ));

        frame.render_widget(gauge, chunks[1]);
    }

    fn render_tab(&self, frame: &mut Frame, area: Rect) /*-> Table<'_>*/
    {
        let rows = match self.selected_tab {
            1 => {
                // SC
                let mut v = Vec::new();
                v.push(
                    Row::new(vec!["Id", "Title", "Songs Count", "Date", "IS"])
                        .style(Style::default().fg(Color::Gray)),
                );
                if let Some(s) = &self.soundcloud {
                    for (i, playlist) in s.iter().enumerate() {
                        let date: DateTime<Utc> = playlist.created_at.parse().unwrap();
                        let mut row = Row::new(vec![
                            playlist.id.to_string(),
                            playlist.title.clone(),
                            [playlist.track_count.to_string(), " songs".to_string()].concat(),
                            format!("{}", date.format("%Y-%m-%d %H:%M")),
                            "NO".to_string(),
                        ]);
                        if self.selected_row == i as i32 {
                            row = row.style(Style::default().bg(Color::Yellow));
                        }
                        v.push(row);
                    }
                }
                v
            }
            _ => Vec::new(),
        };

        // Create the table
        let table = Table::new(
            rows,
            &[
                Constraint::Length(3),      // ID column
                Constraint::Percentage(50), // Playlist name column
                Constraint::Percentage(20), // Song count column
                Constraint::Percentage(30),
                Constraint::Length(2),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title(" Playlists "))
        .style(Style::default().fg(Color::White));

        frame.render_widget(table, area);
    }
}
