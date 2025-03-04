use chrono::{DateTime, TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};
use soundcloud::sobjects::{CloudPlaylist, CloudPlaylists};
use tokio::sync::mpsc::UnboundedSender;

use crate::component::table::SmartTable;
use crate::sync::{DBPlaylist, YTPlaylist};
use crate::{screen::AppScreen, sync::AppEvent, theme::Theme, AppState};

pub struct MainScreen {
    mode: bool,
    selected_tab: i8,
    pl_table: SmartTable,
    song_table: SmartTable,
    tab_titles: Vec<String>,
    youtube: Option<Vec<YTPlaylist>>,
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
            KeyCode::F(5) => self.download_row(),
            KeyCode::F(8) => self.remove_row(),
            KeyCode::F(9) => self.remove_completely(),
            KeyCode::Tab => self.switch_mode(),
            KeyCode::F(4) => {
                let _ = self
                    .sender
                    .send(AppEvent::SwitchScreen(AppState::FileSystem));
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, _theme: &Theme) {
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
            "<TAB> SWITCH PANEL".bold(),
            " | ".dark_gray(),
            "<F4> FS MODE".bold(),
            " | ".dark_gray(),
            "<F5> DOWNLOAD".bold(),
            " | ".dark_gray(),
            "<F8> DEL".bold(),
            " | ".dark_gray(),
            "<F9> DEL REC".bold(),
            " | ".dark_gray(),
            "<Q> QUIT".bold(),
        ]))
        .centered();
        frame.render_widget(status_bar, chunks[2]);
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl MainScreen {
    pub fn new(sender: UnboundedSender<AppEvent>) -> Self {
        MainScreen {
            mode: false,
            pl_table: SmartTable::default(),
            song_table: SmartTable::default(),
            soundcloud: None,
            youtube: None,
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

    fn switch_mode(&mut self) {
        self.set_mode(!self.mode);
    }

    fn set_mode(&mut self, mode: bool) {
        self.mode = mode;
        self.pl_table.set_checked(!self.mode);
        self.song_table.set_checked(self.mode);
    }

    fn next_tab(&mut self) {
        self.selected_tab = std::cmp::min(
            self.selected_tab + 1,
            (self.tab_titles.len() - 1).try_into().unwrap(),
        );
        self.update_tables();
    }

    fn previous_tab(&mut self) {
        self.selected_tab = std::cmp::max(0, self.selected_tab - 1);
        self.update_tables();
    }

    fn previous_row(&mut self) {
        match self.mode {
            true => self.song_table.previous_row(),
            false => {
                self.pl_table.previous_row();
                self.update_songs();
            }
        }
    }

    fn next_row(&mut self) {
        match self.mode {
            true => self.song_table.next_row(),
            false => {
                self.pl_table.next_row();
                self.update_songs();
            }
        }
    }

    fn remove_row(&mut self) {
        if self.selected_tab != 2 {
            return;
        }
        let pl_id = self
            .playlists
            .as_ref()
            .unwrap()
            .get(self.pl_table.selected_row())
            .unwrap()
            .id;
        match self.mode {
            false => {
                let _ = self.sender.send(AppEvent::RemovePlaylist((pl_id, false)));
            }
            true => {
                let track_id = self
                    .playlists
                    .as_ref()
                    .unwrap()
                    .get(self.pl_table.selected_row())
                    .unwrap()
                    .tracks
                    .get(self.song_table.selected_row())
                    .unwrap()
                    .data
                    .unique_id;

                let _ = self
                    .sender
                    .send(AppEvent::RemoveTrackFromPlaylist((track_id, pl_id)));
            }
        }
    }

    fn remove_completely(&mut self) {
        if self.selected_tab != 2 {
            return;
        }
        match self.mode {
            false => {
                let pl_id = self
                    .playlists
                    .as_ref()
                    .unwrap()
                    .get(self.pl_table.selected_row())
                    .unwrap()
                    .id;

                let _ = self.sender.send(AppEvent::RemovePlaylist((pl_id, true)));
            }
            true => {
                let track = self
                    .playlists
                    .as_ref()
                    .unwrap()
                    .get(self.pl_table.selected_row())
                    .unwrap()
                    .tracks
                    .get(self.song_table.selected_row())
                    .unwrap()
                    .clone();
                let _ = self
                    .sender
                    .send(AppEvent::RemoveTrack(track.data.unique_id));
            }
        }
    }

    fn download_row(&mut self) {
        match self.selected_tab {
            0 => {
                // YT
                match self.mode {
                    false => {
                        let playlist = self
                            .youtube
                            .as_ref()
                            .unwrap()
                            .get(self.pl_table.selected_row())
                            .unwrap()
                            .clone();

                        let _ = self.sender.send(AppEvent::DownloadYTPlaylist(playlist));
                    }
                    true => {
                        let track = self
                            .youtube
                            .as_ref()
                            .unwrap()
                            .get(self.pl_table.selected_row())
                            .unwrap()
                            .videos
                            .get(self.song_table.selected_row())
                            .unwrap()
                            .clone();

                        let _ = self.sender.send(AppEvent::DownloadYTTrack(track));
                    }
                }
            }
            1 => {
                // SC
                match self.mode {
                    false => {
                        let playlist = self
                            .soundcloud
                            .as_ref()
                            .unwrap()
                            .get(self.pl_table.selected_row())
                            .unwrap()
                            .clone();
                        let _ = self.sender.send(AppEvent::DownloadPlaylist(playlist));
                    }
                    true => {
                        let track = self
                            .soundcloud
                            .as_ref()
                            .unwrap()
                            .get(self.pl_table.selected_row())
                            .unwrap()
                            .tracks
                            .get(self.song_table.selected_row())
                            .unwrap()
                            .clone();
                        let _ = self.sender.send(AppEvent::DownloadTrack(track));
                    }
                }
            }
            _ => {}
        }
    }

    pub fn set_youtube_playlists(&mut self, pls: Vec<YTPlaylist>) {
        self.youtube = Some(pls);
        if self.selected_tab == 0 {
            self.update_tables();
        }
    }

    pub fn set_soundcloud_playlists(&mut self, pl: CloudPlaylists) {
        self.soundcloud = Some(pl.collection);
        if self.selected_tab == 1 {
            self.update_tables();
        }
    }

    pub fn set_itunes(&mut self, pl: Vec<DBPlaylist>) {
        self.playlists = Some(pl);
        if self.selected_tab == 2 {
            self.update_tables();
        }
    }

    fn update_tables(&mut self) {
        self.set_mode(false);

        self.pl_table = SmartTable::new(
            ["Id", "Title", "Songs Count", "Date", "IS"]
                .iter_mut()
                .map(|s| s.to_string())
                .collect(),
            [
                Constraint::Length(3),      // ID column
                Constraint::Percentage(50), // Playlist name column
                Constraint::Percentage(20), // Song count column
                Constraint::Percentage(30),
                Constraint::Length(2),
            ]
            .to_vec(),
        );

        let data = match self.selected_tab {
            0 => {
                if let Some(yt) = &self.youtube {
                    yt.iter()
                        .map(|playlist| {
                            vec![
                                0.to_string(),
                                playlist.title.clone(),
                                [playlist.videos.len().to_string(), " songs".to_string()].concat(),
                                String::new(),
                                "NO".to_string(),
                            ]
                        })
                        .collect::<Vec<Vec<String>>>()
                } else {
                    Vec::new()
                }
            }
            1 => {
                if let Some(sc) = &self.soundcloud {
                    sc.iter()
                        .map(|playlist| {
                            let date: DateTime<Utc> = playlist.created_at.parse().unwrap();
                            vec![
                                playlist.id.to_string(),
                                playlist.title.clone(),
                                [playlist.track_count.to_string(), " songs".to_string()].concat(),
                                format!("{}", date.format("%Y-%m-%d %H:%M")),
                                "NO".to_string(),
                            ]
                        })
                        .collect::<Vec<Vec<String>>>()
                } else {
                    Vec::new()
                }
            }
            2 => {
                if let Some(it) = &self.playlists {
                    it.iter()
                        .map(|playlist| {
                            let date = Utc.timestamp_millis_opt(playlist.timestamp as i64).unwrap();
                            vec![
                                playlist.id.to_string(),
                                playlist.title.clone(),
                                playlist.tracks.len().to_string(),
                                format!("{}", date.format("%Y-%m-%d %H:%M")),
                                "YES".to_string(),
                            ]
                        })
                        .collect::<Vec<Vec<String>>>()
                } else {
                    Vec::new()
                }
            }
            _ => {
                self.pl_table = SmartTable::default();
                Vec::new()
            }
        };
        self.pl_table.set_data(data);
        self.pl_table.set_title("Playlists".to_string());
        self.update_songs();
    }

    fn update_songs(&mut self) {
        let constraints = [
            Constraint::Length(3),      // ID column
            Constraint::Percentage(50), // Playlist name column
            Constraint::Percentage(20), // Song count column
            Constraint::Length(5),
            Constraint::Min(0),
        ]
        .to_vec();

        match self.selected_tab {
            0 => {
                self.song_table = SmartTable::new(
                    ["Id", "Title", "Artist", "Duration", ""]
                        .iter_mut()
                        .map(|s| s.to_string())
                        .collect(),
                    constraints,
                );
                self.set_mode(self.mode);

                if let Some(pls) = &self.youtube {
                    let y = &pls.get(self.pl_table.selected_row()).unwrap().videos;
                    let data = y
                        .iter()
                        .map(|video| {
                            vec![
                                video.videoId.clone(),
                                video.title.clone(),
                                video.publisher.clone(),
                                video.lengthSeconds.to_string(),
                                String::new(),
                            ]
                        })
                        .collect::<Vec<Vec<String>>>();

                    self.song_table.set_data(data);
                }
                self.song_table.set_title(" Songs ".to_string());
            }
            1 => {
                self.song_table = SmartTable::new(
                    ["Id", "Title", "Artist", "Duration", "Genre"]
                        .iter_mut()
                        .map(|s| s.to_string())
                        .collect(),
                    constraints,
                );
                self.set_mode(self.mode);

                if let Some(pls) = &self.soundcloud {
                    let s = &pls.get(self.pl_table.selected_row()).unwrap().tracks;
                    let data = s
                        .iter()
                        .map(|track| {
                            vec![
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
                            ]
                        })
                        .collect::<Vec<Vec<String>>>();

                    self.song_table.set_data(data);
                }
                self.song_table.set_title(" Songs ".to_string());
            }
            2 => {
                self.song_table = SmartTable::new(
                    ["Id", "Title", "Artist", "Bitrate", "Genre"]
                        .iter_mut()
                        .map(|s| s.to_string())
                        .collect(),
                    constraints,
                );
                self.set_mode(self.mode);

                if let Some(pls) = &self.playlists {
                    let s = &pls.get(self.pl_table.selected_row()).unwrap().tracks;
                    let data = s
                        .iter()
                        .map(|track| {
                            vec![
                                track.data.unique_id.to_string(),
                                track.get_title(),
                                track.get_artist(),
                                track.data.bitrate.to_string(),
                                track.get_genre(),
                            ]
                        })
                        .collect::<Vec<Vec<String>>>();

                    self.song_table.set_data(data);
                }

                self.song_table.set_title(" Songs ".to_string());
            }
            _ => {
                self.song_table = SmartTable::default();
                self.set_mode(self.mode);
            }
        }
    }

    fn render_tab(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(30), // Playlists
                Constraint::Min(0),         // Tracks
            ])
            .split(area);

        self.pl_table.render(frame, chunks[0]);
        self.song_table.render(frame, chunks[1]);
    }
}
