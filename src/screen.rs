use std::any::Any;

use crossterm::event::KeyEvent;
use ratatui::Frame;

use crate::theme::Theme;

pub trait AppScreen {
    fn handle_key_event(&mut self, key_event: KeyEvent);

    fn render(&self, frame: &mut Frame, theme: &Theme);

    fn as_any(&mut self) -> &mut dyn Any;
}
