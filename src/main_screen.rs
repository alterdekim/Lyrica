use chrono::{DateTime, TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table, Tabs},
    Frame,
};
use soundcloud::sobjects::{CloudPlaylist, CloudPlaylists};
use tokio::sync::mpsc::UnboundedSender;

use crate::{db::Track, screen::AppScreen, sync::AppEvent, theme::Theme};
use crate::db::DBPlaylist;

pub struct MainScreen {
    mode: bool,
    selected_tab: i8,
    selected_playlist: i32,
    selected_song: i32,
    max_pls: i32,
    max_songs: i32,
    tab_titles: Vec<String>,
    soundcloud: Option<Vec<CloudPlaylist>>,
    playlists: Option<Vec<DBPlaylist>>,
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
            KeyCode::Tab => self.switch_mode(),
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, theme: &Theme) {
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
                .fg(Color::LightBlue)
                .add_modifier(Modifier::BOLD),
        )
        .select(self.selected_tab as usize)
        .style(Style::default().fg(Color::Black));

        frame.render_widget(tabs, chunks[0]);

        self.render_tab(frame, chunks[1]);

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
            mode: false,
            selected_playlist: 0,
            selected_song: 0,
            max_pls: 0,
            max_songs: 0,
            soundcloud: None,
            playlists: None,
            selected_tab: 0,
            tab_titles: vec![
                "YouTube".to_string(),
                "SoundCloud".to_string(),
                "iPod".to_string(),
                "Settings".to_string(),
            ],
            sender,
        }
    }

    fn update_max_rows(&mut self) {
        self.selected_song = 0;
        self.selected_playlist = 0;
        self.max_songs = 0;
        self.max_pls = match self.selected_tab {
            1 => self.soundcloud.as_deref().unwrap_or(&[]).len(),
            2 => self.playlists.as_deref().unwrap_or(&[]).len(),
            _ => 0,
        }
        .try_into()
        .unwrap();
        self.update_max_songs();
    }

    fn update_max_songs(&mut self) {
        if self.max_pls > 0 {
            self.max_songs = match self.selected_tab {
                1 => self
                    .soundcloud
                    .as_deref()
                    .unwrap()
                    .get(self.selected_playlist as usize)
                    .unwrap()
                    .tracks
                    .len(),
                _ => 0,
            }
            .try_into()
            .unwrap();

            self.selected_song = 0;
        }
    }

    fn switch_mode(&mut self) {
        self.mode = !self.mode;
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
        match self.mode {
            true => self.selected_song = std::cmp::max(0, self.selected_song - 1),
            false => {
                self.selected_playlist = std::cmp::max(0, self.selected_playlist - 1);
                self.update_max_songs();
            }
        }
    }

    fn next_row(&mut self) {
        match self.mode {
            true => self.selected_song = std::cmp::min(self.selected_song + 1, self.max_songs - 1),
            false => {
                self.selected_playlist =
                    std::cmp::min(self.selected_playlist + 1, self.max_pls - 1);
                self.update_max_songs();
            }
        }
    }

    fn download_row(&mut self) {
        if self.selected_tab == 1 {
            // SC
            let playlist = self
                .soundcloud
                .as_ref()
                .unwrap()
                .get(self.selected_playlist as usize)
                .unwrap()
                .clone();
            let _ = self.sender.send(AppEvent::DownloadPlaylist(playlist));
        }
    }

    pub fn set_soundcloud_playlists(&mut self, pl: CloudPlaylists) {
        self.soundcloud = Some(pl.collection);
        if self.selected_tab == 1 {
            self.update_max_rows();
        }
    }
    
    pub fn set_itunes(&mut self, pl: Vec<DBPlaylist>) {
        self.playlists = Some(pl);
        if self.selected_tab == 2 {
            self.update_max_rows();
        }
    }

    fn render_tab(&self, frame: &mut Frame, area: Rect) {
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
                        if self.selected_playlist == i as i32 {
                            row = row.style(Style::default().bg(Color::LightBlue).fg(Color::White));
                        }
                        v.push(row);
                    }
                }
                v
            }
            2 => {
                // local
                let mut v = Vec::new();
                v.push(
                    Row::new(vec!["Id", "Title", "Songs Count", "Date", "IS"])
                        .style(Style::default().fg(Color::Gray)),
                );
                if let Some(s) = &self.playlists {
                    for (i, playlist) in s.iter().enumerate() {
                        let date = Utc.timestamp_millis_opt(playlist.timestamp as i64).unwrap();
                        let mut row = Row::new(vec![
                            playlist.persistent_playlist_id.to_string(),
                            "".to_string(),
                            playlist.tracks.len().to_string(),
                            format!("{}", date.format("%Y-%m-%d %H:%M")),
                            "YES".to_string(),
                        ]);
                        if self.selected_playlist == i as i32 {
                            row = row.style(Style::default().bg(Color::LightBlue).fg(Color::White));
                        }
                        v.push(row);
                    }
                }
                v
            }
            _ => Vec::new(),
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30), // Playlists
                Constraint::Min(0),         // Tracks
            ])
            .split(area);

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
        .style(Style::default().fg(Color::Black));

        frame.render_widget(table, chunks[0]);

        let rows = match self.selected_tab {
            1 => {
                // sc
                let mut v = Vec::new();
                v.push(
                    Row::new(vec!["Id", "Title", "Artist", "Duration", "Genre"])
                        .style(Style::default().fg(Color::Gray)),
                );
                if let Some(pls) = &self.soundcloud {
                    let s = &pls.get(self.selected_playlist as usize).unwrap().tracks;
                    for (i, track) in s.iter().enumerate() {
                        let mut row = Row::new(vec![
                            track.id.to_string(),
                            track.title.as_deref().unwrap().to_string(),
                            track
                                .user
                                .clone()
                                .unwrap()
                                .username
                                .unwrap_or(track.user.as_ref().unwrap().permalink.clone()),
                            track.duration.unwrap_or(0).to_string(),
                            track.genre.as_ref().unwrap_or(&String::new()).to_string(),
                        ]);
                        if self.selected_song == i as i32 {
                            row = row.style(Style::default().bg(Color::LightBlue).fg(Color::White));
                        }
                        v.push(row);
                    }
                }
                v
            },
            2 => {
                // local
                let mut v = Vec::new();
                v.push(
                    Row::new(vec!["Id", "Title", "Artist", "Bitrate", "Genre"])
                        .style(Style::default().fg(Color::Gray)),
                );
                if let Some(pls) = &self.playlists {
                    let s = &pls.get(self.selected_playlist as usize).unwrap().tracks;
                    for (i, track) in s.iter().enumerate() {
                        let mut row = Row::new(vec![
                            track.unique_id.to_string(),
                            track.title.clone(),
                            track.artist.clone(),
                            track.bitrate.to_string(),
                            track.genre.clone(),
                        ]);
                        if self.selected_song == i as i32 {
                            row = row.style(Style::default().bg(Color::LightBlue).fg(Color::White));
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
                Constraint::Length(5),
                Constraint::Min(0),
            ],
        )
        .block(Block::default().borders(Borders::ALL).title(" Songs "))
        .style(Style::default().fg(Color::Black));

        frame.render_widget(table, chunks[1]);
    }
}
