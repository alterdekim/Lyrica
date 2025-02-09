use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{buffer::Buffer, layout::Rect, style::{Color, Stylize}, text::Line, widgets::{Tabs, Widget}};
use soundcloud::sobjects::CloudPlaylists;
use strum::IntoEnumIterator;

use crate::tabs::SelectedTab;

#[derive(Debug, Clone)]
pub struct MainScreen {
    pub selected_tab: SelectedTab,
    pub soundcloud: Option<CloudPlaylists>
}

impl MainScreen {
    pub fn new() -> Self {
        MainScreen { selected_tab: SelectedTab::Playlists, soundcloud: None }
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('l') | KeyCode::Right => self.next_tab(),
            KeyCode::Char('h') | KeyCode::Left => self.previous_tab(),
            _ => {}
        }
    }

    pub fn render_title(area: Rect, buf: &mut Buffer) {
        "Lyrica".bold().render(area, buf);
    }
    
    pub fn render_footer(area: Rect, buf: &mut Buffer) {
        Line::raw("◄ ► to change tab | <Q> to quit")
            .centered()
            .render(area, buf);
    }

    pub fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = SelectedTab::iter().map(SelectedTab::title);
        let highlight_style = (Color::default(), self.selected_tab.palette().c700);
        let selected_tab_index = self.selected_tab.to_usize();
        Tabs::new(titles)
            .highlight_style(highlight_style)
            .select(selected_tab_index)
            .padding("", "")
            .divider(" ")
            .render(area, buf);
    }

    fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }
}