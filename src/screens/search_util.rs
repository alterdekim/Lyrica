use crate::component::table::SmartTable;
use crate::screens::AppScreen;
use crate::sync::AppEvent;
use crate::AppState;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Line, Stylize};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::any::Any;
use tokio::sync::mpsc::UnboundedSender;

fn table() -> SmartTable {
    SmartTable::new(
        ["Id", "Title", "Artist", "Album", "Genre", "Type"]
            .iter_mut()
            .map(|s| s.to_string())
            .collect(),
        vec![
            Constraint::Length(16),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Length(10),
        ],
    )
}

pub struct SearchScreen {
    table: SmartTable,
    sender: UnboundedSender<AppEvent>,
    entries: Option<Vec<SearchEntry>>,
}

impl AppScreen for SearchScreen {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Up => self.table.previous_row(),
            KeyCode::Down => self.table.next_row(),
            KeyCode::Esc => {
                let _ = self
                    .sender
                    .send(AppEvent::SwitchScreen(AppState::MainScreen));
            }
            KeyCode::F(8) => self.remove_row(),
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame) {
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
            "<ESC> GO BACK".bold(),
            " | ".dark_gray(),
            "<F10> QUIT".bold(),
        ]))
        .centered();
        frame.render_widget(status_bar, chunks[1]); // Render into third chunk
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

impl SearchScreen {
    pub fn new(sender: UnboundedSender<AppEvent>) -> Self {
        Self {
            table: table(),
            sender,
            entries: None,
        }
    }

    fn remove_row(&mut self) {
        if let Some(entries) = &self.entries {
            let id = entries[self.table.selected_row()].id;
            let is_pl = entries[self.table.selected_row()].is_playlist;

            if is_pl {
                let _ = self.sender.send(AppEvent::RemovePlaylist((id, false)));
                return;
            }
            let _ = self.sender.send(AppEvent::RemoveTrack(id as u32));
        }
    }

    pub fn show_search(&mut self, entries: Vec<SearchEntry>) {
        self.table = table();

        self.table.set_title(String::from("Search results"));

        let data = entries
            .iter()
            .map(|i| {
                vec![
                    i.id.to_string(),
                    i.title.clone(),
                    i.artist.clone(),
                    i.album.clone(),
                    i.genre.clone(),
                    if i.is_playlist { "PL" } else { "TR" }.to_string(),
                ]
            })
            .collect();

        self.table.set_data(data);

        self.entries = Some(entries);
    }

    fn render_main(&self, frame: &mut Frame, area: Rect) {
        self.table.render(frame, area);
    }
}

pub struct SearchEntry {
    pub id: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub is_playlist: bool,
}

impl SearchEntry {
    pub fn track(id: u64, title: String, artist: String, album: String, genre: String) -> Self {
        Self {
            id,
            title,
            album,
            artist,
            genre,
            is_playlist: false,
        }
    }

    pub fn playlist(id: u64, entry: String) -> Self {
        Self {
            id,
            title: entry,
            is_playlist: true,
            album: String::default(),
            artist: String::default(),
            genre: String::default(),
        }
    }
}
