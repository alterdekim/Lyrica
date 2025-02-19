use crate::component::table::SmartTable;
use crate::sync::AppEvent;
use crate::{screen::AppScreen, theme::Theme, AppState};
use chrono::{DateTime, Utc};
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::prelude::{Line, Stylize};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::cmp::Ordering;
use std::ffi::OsStr;
use std::fs::DirEntry;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

pub struct FileSystem {
    files: Vec<DirEntry>,
    table: SmartTable,
    sender: UnboundedSender<AppEvent>,
}

fn check_extension_comptability(ext: &OsStr) -> bool {
    matches!(
        ext.to_str().unwrap().to_lowercase().as_str(),
        "mp3" | "m4a" | "wav" | "aiff" | "aif" | "aac"
    )
}

impl AppScreen for FileSystem {
    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) {
        match key_event.code {
            KeyCode::Up => self.table.previous_row(),
            KeyCode::Down => self.table.next_row(),
            KeyCode::F(4) => {
                let _ = self
                    .sender
                    .send(AppEvent::SwitchScreen(AppState::MainScreen));
            }
            KeyCode::F(5) => {
                self.download_as_is();
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut Frame, _theme: &Theme) {
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
            "<F4> SWITCH TO NORMAL".bold(),
            " | ".dark_gray(),
            "<F5> SAVE AS IS".bold(),
            " | ".dark_gray(),
            "<F6> SAVE AS PLAYLIST".bold(),
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
    pub fn new(sender: UnboundedSender<AppEvent>) -> Self {
        let table = SmartTable::new(
            ["Name", "Type", "Size", "Modified"]
                .iter_mut()
                .map(|s| s.to_string())
                .collect(),
            vec![
                Constraint::Percentage(50),
                Constraint::Length(5),
                Constraint::Percentage(20),
                Constraint::Percentage(30),
            ],
        );

        let mut a = Self {
            table,
            sender,
            files: Vec::new(),
        };
        a.get_path(dirs::document_dir().unwrap());
        a
    }

    fn get_path(&mut self, p: PathBuf) {
        let paths = std::fs::read_dir(&p).unwrap();
        let mut dir = paths
            .filter_map(|res| res.ok())
            .filter(|p| {
                p.path()
                    .extension()
                    .map_or(false, check_extension_comptability)
                    || p.path().is_dir()
            })
            .collect::<Vec<DirEntry>>();
        dir.sort_by(|a, _b| {
            if a.file_type().unwrap().is_dir() {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        });

        let data = dir
            .iter()
            .map(|entry| {
                let datetime: DateTime<Utc> = entry.metadata().unwrap().modified().unwrap().into();
                let datetime = datetime.format("%d/%m/%Y %T").to_string();
                let size = entry.metadata().unwrap().size().to_string();
                let file_type = entry.file_type().unwrap().is_file().to_string();
                vec![
                    entry.file_name().to_str().unwrap().to_string(),
                    file_type,
                    size,
                    datetime,
                ]
            })
            .collect::<Vec<Vec<String>>>();

        self.files = dir;

        self.table.set_data(data);
        self.table
            .set_title(p.iter().last().unwrap().to_str().unwrap().to_string());
    }

    fn download_as_is(&self) {
        let entry = self.files.get(self.table.selected_row()).unwrap();
        if entry.path().is_dir() {
            todo!("Implement that later");
        } else {
            let _ = self.sender.send(AppEvent::LoadFromFS(entry.path()));
        }
    }

    fn render_main(&self, frame: &mut Frame, area: Rect) {
        self.table.render(frame, area);
    }
}
