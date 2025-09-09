use chrono::{DateTime, TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::Clear;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};
use soundcloud::sobjects::CloudPlaylist;
use std::collections::HashMap;
use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount as EnumCountMacro, EnumIter};
use tokio::sync::mpsc::UnboundedSender;

use crate::component::table::SmartTable;
use crate::sync::{DBPlaylist, YTPlaylist};
use crate::{screens::AppScreen, sync::AppEvent, AppState};

fn rect_layout(direction: Direction, percent: u16) -> Layout {
    Layout::default().direction(direction).constraints(
        [
            Constraint::Percentage((100 - percent) / 2),
            Constraint::Percentage(percent),
            Constraint::Percentage((100 - percent) / 2),
        ]
        .as_ref(),
    )
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = rect_layout(Direction::Vertical, percent_y).split(r);
    let vertical_chunk = popup_layout[1];
    let horizontal_layout = rect_layout(Direction::Horizontal, percent_x).split(vertical_chunk);
    horizontal_layout[1]
}

#[derive(Debug, EnumCountMacro, EnumIter, Eq, Hash, PartialEq, Clone, Copy)]
pub enum TabType {
    Youtube,
    Soundcloud,
    Playlists,
}

impl From<i8> for TabType {
    fn from(value: i8) -> Self {
        match value {
            0 => TabType::Youtube,
            1 => TabType::Soundcloud,
            _ => TabType::Playlists,
        }
    }
}

impl From<TabType> for String {
    fn from(value: TabType) -> Self {
        match value {
            TabType::Youtube => "YouTube",
            TabType::Soundcloud => "SoundCloud",
            TabType::Playlists => "Local Playlists",
        }
        .to_string()
    }
}

pub enum TabContent {
    Youtube(Vec<YTPlaylist>),
    SoundCloud(Vec<CloudPlaylist>),
    Playlists(Vec<DBPlaylist>),
}

pub struct MainScreen {
    mode: bool,
    selected_tab: i8,
    pl_table: SmartTable,
    song_table: SmartTable,
    tab_content: HashMap<TabType, TabContent>,
    sender: UnboundedSender<AppEvent>,
    show_popup: bool,
    popup_input: String,
}

impl AppScreen for MainScreen {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Right => self.next_tab(),
            KeyCode::Left => self.previous_tab(),
            KeyCode::Up => self.previous_row(),
            KeyCode::Down => self.next_row(),
            KeyCode::F(2) => self.search_popup(),
            KeyCode::F(5) => self.download_row(),
            KeyCode::F(8) => self.remove_row(),
            KeyCode::F(9) => self.remove_completely(),
            KeyCode::Tab => self.switch_mode(),
            KeyCode::F(4) => {
                let _ = self
                    .sender
                    .send(AppEvent::SwitchScreen(AppState::FileSystem));
            }
            KeyCode::Char(c) => {
                if self.show_popup && (c.is_alphanumeric() || c.is_ascii_whitespace()) {
                    self.popup_input.push(c);
                }
            }
            KeyCode::Esc => {
                if self.show_popup {
                    self.show_popup = false;
                    self.popup_input = String::default();
                }
            }
            KeyCode::Backspace => {
                if self.show_popup {
                    self.popup_input.pop();
                }
            }
            KeyCode::Enter => {
                if self.show_popup {
                    self.show_popup = false;
                    let _ = self
                        .sender
                        .send(AppEvent::SearchFor(self.popup_input.clone()));
                    self.popup_input = String::default();
                }
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame) {
        let size = frame.area();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Tabs
                Constraint::Min(0),    // Main content area
                Constraint::Length(1), // Status bar
            ])
            .split(frame.area());

        let tabs = Tabs::new(
            TabType::iter()
                .map(|t| Span::raw(String::from(t).clone()))
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
        let status_bar = Paragraph::new(Line::from(match TabType::from(self.selected_tab) {
            TabType::Youtube | TabType::Soundcloud => {
                vec![
                    "<F2> SEARCH".bold(),
                    " | ".dark_gray(),
                    "<F4> IMPORT".bold(),
                    " | ".dark_gray(),
                    "<F5> DOWNLOAD".bold(),
                    " | ".dark_gray(),
                    "<F10> QUIT".bold(),
                ]
            }
            TabType::Playlists => {
                vec![
                    "<F2> SEARCH".bold(),
                    " | ".dark_gray(),
                    "<F4> IMPORT".bold(),
                    " | ".dark_gray(),
                    "<F8> REMOVE".bold(),
                    " | ".dark_gray(),
                    "<F9> DELETE".bold(),
                    " | ".dark_gray(),
                    "<F10> QUIT".bold(),
                ]
            }
        }))
        .centered();
        frame.render_widget(status_bar, chunks[2]);

        if self.show_popup {
            // Get a centered rect (50% width, 30% height)
            let popup_area = centered_rect(30, 10, size);

            // Clear background behind the popup
            frame.render_widget(Clear, popup_area);

            // Draw the popup block
            let block = Block::default()
                .title(" Search ")
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded);
            let inner = block.inner(popup_area);
            frame.render_widget(block, popup_area);

            // Example: input field
            let paragraph = Paragraph::new(self.popup_input.as_str());
            frame.render_widget(paragraph, inner);
        }
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
            selected_tab: 0,
            sender,
            show_popup: false,
            popup_input: String::default(),
            tab_content: HashMap::new(),
        }
    }

    fn switch_mode(&mut self) {
        self.set_mode(!self.mode);
    }

    fn search_popup(&mut self) {
        self.show_popup = true;
    }

    fn set_mode(&mut self, mode: bool) {
        self.mode = mode;
        self.pl_table.set_checked(!self.mode);
        self.song_table.set_checked(self.mode);
    }

    fn next_tab(&mut self) {
        self.selected_tab = std::cmp::min(
            self.selected_tab + 1,
            (TabType::COUNT - 1).try_into().unwrap(),
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
        if let Some(TabContent::Playlists(playlists)) =
            self.tab_content.get(&TabType::from(self.selected_tab))
        {
            let pl_id = playlists.get(self.pl_table.selected_row()).unwrap().id;
            match self.mode {
                false => {
                    let _ = self.sender.send(AppEvent::RemovePlaylist((pl_id, false)));
                }
                true => {
                    let track_id = playlists
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
    }

    fn remove_completely(&mut self) {
        if let Some(TabContent::Playlists(playlists)) =
            self.tab_content.get(&TabType::from(self.selected_tab))
        {
            match self.mode {
                false => {
                    let pl_id = playlists.get(self.pl_table.selected_row()).unwrap().id;

                    let _ = self.sender.send(AppEvent::RemovePlaylist((pl_id, true)));
                }
                true => {
                    let track = playlists
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
    }

    fn download_row(&mut self) {
        match self.tab_content.get(&TabType::from(self.selected_tab)) {
            Some(TabContent::Youtube(youtube)) => match self.mode {
                false => {
                    let playlist = youtube.get(self.pl_table.selected_row()).unwrap().clone();

                    let _ = self.sender.send(AppEvent::DownloadYTPlaylist(playlist));
                }
                true => {
                    let track = youtube
                        .get(self.pl_table.selected_row())
                        .unwrap()
                        .videos
                        .get(self.song_table.selected_row())
                        .unwrap()
                        .clone();

                    let _ = self.sender.send(AppEvent::DownloadYTTrack(track));
                }
            },
            Some(TabContent::SoundCloud(soundcloud)) => match self.mode {
                false => {
                    let playlist = soundcloud
                        .get(self.pl_table.selected_row())
                        .unwrap()
                        .clone();
                    let _ = self.sender.send(AppEvent::DownloadPlaylist(playlist));
                }
                true => {
                    let track = soundcloud
                        .get(self.pl_table.selected_row())
                        .unwrap()
                        .tracks
                        .get(self.song_table.selected_row())
                        .unwrap()
                        .clone();
                    let _ = self.sender.send(AppEvent::DownloadTrack(track));
                }
            },
            _ => {}
        }
    }

    pub fn set_playlists(&mut self, tab: TabType, content: TabContent) {
        self.tab_content.insert(tab, content);
        if TabType::from(self.selected_tab) == tab {
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

        if let Some(content) = self.tab_content.get(&TabType::from(self.selected_tab)) {
            let data = match content {
                TabContent::Youtube(yt) => yt
                    .iter()
                    .map(|playlist| {
                        vec![
                            0.to_string(),
                            playlist.title.clone(),
                            [playlist.videos.len().to_string(), " songs".to_string()].concat(),
                            String::new(),
                            "NO".to_string(),
                        ]
                    })
                    .collect::<Vec<Vec<String>>>(),
                TabContent::SoundCloud(sc) => sc
                    .iter()
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
                    .collect::<Vec<Vec<String>>>(),
                TabContent::Playlists(it) => it
                    .iter()
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
                    .collect::<Vec<Vec<String>>>(),
            };

            self.pl_table.set_data(data);
            self.pl_table.set_title("Playlists".to_string());
        }
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

        match self.tab_content.get(&TabType::from(self.selected_tab)) {
            Some(TabContent::Youtube(pls)) => {
                self.song_table = SmartTable::new(
                    ["Id", "Title", "Artist", "Duration", ""]
                        .iter_mut()
                        .map(|s| s.to_string())
                        .collect(),
                    constraints,
                );

                if let Some(ypl) = &pls.get(self.pl_table.selected_row()) {
                    let y = ypl.videos.clone();
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
            Some(TabContent::SoundCloud(pls)) => {
                self.song_table = SmartTable::new(
                    ["Id", "Title", "Artist", "Duration", "Genre"]
                        .iter_mut()
                        .map(|s| s.to_string())
                        .collect(),
                    constraints,
                );

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

                self.song_table.set_title(" Songs ".to_string());
            }
            Some(TabContent::Playlists(pls)) => {
                self.song_table = SmartTable::new(
                    ["Id", "Title", "Artist", "Bitrate", "Genre"]
                        .iter_mut()
                        .map(|s| s.to_string())
                        .collect(),
                    constraints,
                );

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

                self.song_table.set_title(" Songs ".to_string());
            }
            _ => {
                self.song_table = SmartTable::default();
            }
        }
        self.set_mode(self.mode);
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
