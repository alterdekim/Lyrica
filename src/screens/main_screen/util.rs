use crate::sync::{DBPlaylist, YTPlaylist};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use soundcloud::sobjects::CloudPlaylist;
use strum_macros::{EnumCount as EnumCountMacro, EnumIter};

fn rect_layout(direction: Direction, percent: u16) -> Layout {
    Layout::default().direction(direction).constraints(
        [
            Constraint::Percentage((100 - percent) / 2),
            Constraint::Percentage(percent),
            Constraint::Percentage((100 - percent) / 2),
        ]
        .as_ref(),
    )
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = rect_layout(Direction::Vertical, percent_y).split(r);
    let vertical_chunk = popup_layout[1];
    let horizontal_layout = rect_layout(Direction::Horizontal, percent_x).split(vertical_chunk);
    horizontal_layout[1]
}

#[derive(Debug, EnumCountMacro, EnumIter, Eq, Hash, PartialEq, Clone, Copy)]
pub enum TabType {
    Youtube,
    Soundcloud,
    Playlists,
}

impl From<i8> for TabType {
    fn from(value: i8) -> Self {
        match value {
            0 => TabType::Youtube,
            1 => TabType::Soundcloud,
            _ => TabType::Playlists,
        }
    }
}

impl From<TabType> for String {
    fn from(value: TabType) -> Self {
        match value {
            TabType::Youtube => "YouTube",
            TabType::Soundcloud => "SoundCloud",
            TabType::Playlists => "Local Playlists",
        }
        .to_string()
    }
}

pub enum TabContent {
    Youtube(Vec<YTPlaylist>),
    SoundCloud(Vec<CloudPlaylist>),
    Playlists(Vec<DBPlaylist>),
}
