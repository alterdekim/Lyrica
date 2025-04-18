use crate::component::table::SmartTable;
use crate::sync::AppEvent;
use crate::{screen::AppScreen, AppState};
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
    current_path: PathBuf,
    table: SmartTable,
    sender: UnboundedSender<AppEvent>,
}

fn check_extension_compatibility(ext: &str) -> bool {
    matches!(
        ext.to_lowercase().as_str(),
        "mp3" | "m4a" | "wav" | "aiff" | "aif"
    )
}

fn get_extension_from_filename(file_name: Option<&OsStr>) -> String {
    if let Some(fname) = file_name {
        let file_name = fname.to_str().unwrap();
        let index = file_name
            .chars()
            .enumerate()
            .filter(|(_i, c)| *c == '.')
            .map(|(i, _c)| i)
            .last();
        if let Some(index) = index {
            let extension: String = file_name.chars().skip(index).collect();
            return extension;
        }
    }
    "none".to_string()
}

fn list_files_recursively(p: PathBuf) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let paths = std::fs::read_dir(p).unwrap();

    for path in paths {
        if path.is_err() {
            continue;
        }
        let a = path.unwrap().path();
        if a.is_file()
            && check_extension_compatibility(
                a.extension()
                    .map_or(&get_extension_from_filename(a.file_name()), |s| {
                        s.to_str().unwrap()
                    }),
            )
        {
            files.push(a.clone());
        }
        if a.is_dir() {
            files.append(&mut list_files_recursively(a));
        }
    }

    files
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
            KeyCode::F(5) => self.download_as_is(),
            KeyCode::F(6) => self.download_as_playlist(),
            KeyCode::Enter => self.enter_directory(),
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
            "<F4> SWITCH TO NORMAL".bold(),
            " | ".dark_gray(),
            "<F5> SAVE AS IS".bold(),
            " | ".dark_gray(),
            "<F6> SAVE AS PLAYLIST".bold(),
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
            current_path: dirs::document_dir().unwrap(),
        };
        a.get_path(dirs::document_dir().unwrap());
        a
    }

    fn get_path(&mut self, p: PathBuf) {
        self.current_path = p.clone();
        let paths = std::fs::read_dir(&p).unwrap();
        let mut dir = paths
            .filter_map(|res| res.ok())
            .filter(|p| {
                p.path().extension().map_or(false, |s| {
                    check_extension_compatibility(s.to_str().unwrap_or("none"))
                }) || p.path().is_dir()
            })
            .collect::<Vec<DirEntry>>();
        dir.sort_by(|a, b| {
            if a.file_type().unwrap().is_dir() == b.file_type().unwrap().is_dir() {
                let af = a.file_name();
                let bf = b.file_name();
                let ac = af.to_str().unwrap_or("a");
                let bc = bf.to_str().unwrap_or("a");
                return ac.cmp(bc);
            }
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
                let file_type = if entry.file_type().unwrap().is_file() {
                    "FILE"
                } else {
                    "DIR"
                }
                .to_string();
                vec![
                    entry.file_name().to_str().unwrap().to_string(),
                    file_type,
                    size,
                    datetime,
                ]
            })
            .collect::<Vec<Vec<String>>>();

        let data = [vec![vec![String::from("..")]], data].concat();

        self.files = dir;

        self.table.set_data(data);
        self.table
            .set_title(p.iter().last().unwrap().to_str().unwrap().to_string());
    }

    fn download_as_is(&self) {
        if let 1.. = self.table.selected_row() {
            let entry = self.files.get(self.table.selected_row() - 1).unwrap();
            if entry.path().is_dir() {
                let files = list_files_recursively(entry.path());
                let _ = self.sender.send(AppEvent::LoadFromFSVec(files));
            } else {
                let _ = self.sender.send(AppEvent::LoadFromFS(entry.path()));
            }
        }
    }

    fn download_as_playlist(&self) {
        if let 1.. = self.table.selected_row() {
            let entry = self.files.get(self.table.selected_row() - 1).unwrap();
            if entry.path().is_dir() {
                let files = list_files_recursively(entry.path());
                let _ = self.sender.send(AppEvent::LoadFromFSPL((
                    files,
                    entry
                        .path()
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                )));
            }
        }
    }

    fn move_up(&mut self) {
        let p = self.current_path.parent();
        if p.is_none() {
            return;
        }
        let p: PathBuf = p.unwrap().to_path_buf();
        self.get_path(p);
    }

    fn enter_directory(&mut self) {
        match self.table.selected_row() {
            0 => self.move_up(),
            _ => {
                let entry = self.files.get(self.table.selected_row() - 1).unwrap();
                if !entry.path().is_dir() {
                    return;
                }
                self.get_path(entry.path());
            }
        }
    }

    fn render_main(&self, frame: &mut Frame, area: Rect) {
        self.table.render(frame, area);
    }
}
