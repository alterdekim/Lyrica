use crate::{screen::AppScreen, theme::Theme};

pub struct FileSystem {}

impl AppScreen for FileSystem {
    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) {}

    fn render(&self, frame: &mut ratatui::Frame, theme: &Theme) {
        todo!()
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        todo!()
    }
}
