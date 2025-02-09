use ratatui::{buffer::Buffer, layout::Rect, style::{palette::tailwind, Stylize}, symbols, text::Line, widgets::{Block, Padding, Paragraph, Widget}};
use soundcloud::sobjects::CloudPlaylists;
use strum::{AsRefStr, Display, EnumIter, FromRepr, IntoEnumIterator};

use crate::screen::MainScreen;

#[derive(Debug, Default, Clone, Display, FromRepr, EnumIter, AsRefStr)]
pub enum SelectedTab {
    #[default]
    #[strum(to_string = "Playlists")]
    Playlists,
    #[strum(to_string = "Albums")]
    Albums,
    #[strum(to_string = "Soundcloud")]
    Soundcloud(Option<CloudPlaylists>),
    #[strum(to_string = "Youtube")]
    Youtube,
}

impl Widget for SelectedTab {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        match self {
            Self::Albums => self.render_albums(area, buf),
            Self::Playlists => self.render_playlists(area, buf),
            Self::Soundcloud(playlists) => SelectedTab::render_soundcloud(block,area, buf, playlists),
            Self::Youtube => self.render_youtube(area, buf),
        }
    }
}

impl SelectedTab {
    /// Return tab's name as a styled `Line`
    pub fn title(self) -> Line<'static> {
        format!("  {self}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }

    fn render_albums(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Hello, World!")
            .block(self.block())
            .render(area, buf);
    }

    fn render_playlists(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("Welcome to the Ratatui tabs example!")
            .block(self.block())
            .render(area, buf);
    }

    fn render_soundcloud(block: Block<'static>, area: Rect, buf: &mut Buffer, playlists: Option<CloudPlaylists>) {
        Paragraph::new("Your playlists from soundcloud:")
            .block(block)
            .render(area, buf);
    }

    fn render_youtube(self, area: Rect, buf: &mut Buffer) {
        Paragraph::new("I know, these are some basic changes. But I think you got the main idea.")
            .block(self.block())
            .render(area, buf);
    }

    /// A block surrounding the tab's content
    fn block(&self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::THICK)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
    }

    pub fn palette(&self) -> tailwind::Palette {
        match self {
            Self::Albums => tailwind::INDIGO,
            Self::Playlists => tailwind::EMERALD,
            Self::Soundcloud(_) => tailwind::ORANGE,
            Self::Youtube => tailwind::RED,
        }
    }

    pub fn previous(self) -> Self {
        let current_index = self.clone().to_usize();
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index).unwrap_or(self)
    }

    pub fn next(self) -> Self {
        let current_index = self.clone().to_usize();
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }

    pub fn to_usize(self) -> usize {
        SelectedTab::iter().enumerate().find(|(_i, el)| el.as_ref() == self.as_ref()).unwrap().0
    }
}