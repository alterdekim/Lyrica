use std::any::Any;

use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect, Frame};

pub trait AppScreen {
    fn handle_key_event(&mut self, key_event: KeyEvent);

    fn render(&self, frame: &mut Frame);

    fn as_any(&mut self) -> &mut dyn Any;
}