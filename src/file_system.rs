use crate::screen::AppScreen;

pub struct FileSystem {}

impl AppScreen for FileSystem {
    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) {}

    fn render(&self, frame: &mut ratatui::Frame) {
        todo!()
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        todo!()
    }
}
