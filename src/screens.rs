use std::any::Any;

use crossterm::event::KeyEvent;
use ratatui::Frame;

pub mod file_system;
pub mod loading_screen;
pub mod main_screen;
pub mod search_util;
pub mod wait_screen;

pub trait AppScreen {
    fn handle_key_event(&mut self, key_event: KeyEvent);

    fn render(&self, frame: &mut Frame);

    fn as_any(&mut self) -> &mut dyn Any;
}
