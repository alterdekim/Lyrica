use crate::screen::AppScreen;
use color_eyre::owo_colors::OwoColorize;
use ratatui::layout::{Constraint, Direction, Flex, Layout};
use ratatui::widgets::Paragraph;
use ratatui::{
    style::{Style, Stylize},
    text::Line,
    Frame,
};
use throbber_widgets_tui::{ThrobberState, BOX_DRAWING};
use tui_big_text::{BigText, PixelSize};

#[derive(Debug, Clone, Default)]
pub struct WaitScreen {}

impl AppScreen for WaitScreen {
    fn handle_key_event(&mut self, _key_event: crossterm::event::KeyEvent) {}

    fn render(&self, frame: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(33); 3])
            .split(frame.area());

        let simple = throbber_widgets_tui::Throbber::default()
            .label("Searching for your iPod")
            .throbber_set(BOX_DRAWING);

        let bottom =
            Paragraph::new(vec![Line::from(vec![" Quit ".into(), "<Q> ".red().bold()])]).centered();
        let bottom_l =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(chunks[2]);

        let [throbber_l] = Layout::horizontal([Constraint::Length(
            simple.to_line(&ThrobberState::default()).width() as u16,
        )])
        .flex(Flex::Center)
        .areas(bottom_l[0]);

        frame.render_widget(simple, throbber_l);
        frame.render_widget(bottom, bottom_l[1]);

        let title = BigText::builder()
            .pixel_size(PixelSize::Full)
            .style(Style::new().blue())
            .lines(vec!["Lyrica".light_blue().into()])
            .centered()
            .build();

        frame.render_widget(title, chunks[1]);
    }

    fn as_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
