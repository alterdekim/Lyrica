use color_eyre::owo_colors::OwoColorize;
use ratatui::{
    style::{Style, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph},
    Frame,
};

use crate::{screen::AppScreen, theme::Theme};

#[derive(Debug, Clone, Default)]
pub struct WaitScreen {}

impl AppScreen for WaitScreen {
    fn handle_key_event(&mut self, key_event: crossterm::event::KeyEvent) {}

    fn render(&self, frame: &mut Frame, theme: &Theme) {
        let title = Line::from(" Lyrica ".bold());
        let instructions = Line::from(vec![" Quit ".into(), "<Q> ".red().bold()]);
        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::ROUNDED);

        let counter_text = Text::from(vec![Line::from(vec!["Searching for iPod...".into()])]);

        let par = Paragraph::new(counter_text)
            .style(Style::new().bg(theme.background()))
            .centered()
            .block(block);

        frame.render_widget(par, frame.area());
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
