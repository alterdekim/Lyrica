use crate::dlp::DownloadProgress;
use crate::screens::search_util::SearchEntry;
use crate::AppState;
use itunesdb::xobjects::XTrackItem;
use soundcloud::sobjects::{CloudPlaylist, CloudPlaylists, CloudTrack};
use std::path::PathBuf;
use youtube_api::objects::YoutubeVideo;

pub enum AppEvent {
    SearchIPod,
    IPodNotFound,
    ITunesParsed(Vec<DBPlaylist>),
    YoutubeGot(Vec<YTPlaylist>),
    SoundcloudGot(CloudPlaylists),
    DownloadPlaylist(CloudPlaylist),
    DownloadTrack(CloudTrack),
    DownloadYTPlaylist(YTPlaylist),
    DownloadYTTrack(YoutubeVideo),
    CurrentProgress(DownloadProgress),
    OverallProgress((u32, u32, ratatui::style::Color)),
    ArtworkProgress((u32, u32)),
    SwitchScreen(AppState),
    LoadFromFS(PathBuf),
    LoadFromFSVec(Vec<PathBuf>),
    LoadFromFSPL((Vec<PathBuf>, String)),
    RemoveTrack(u32),
    RemovePlaylist((u64, bool)),
    RemoveTrackFromPlaylist((u32, u64)),
    SearchFor(String),
    SearchShow(Vec<SearchEntry>),
}

pub struct DBPlaylist {
    pub id: u64,
    pub title: String,
    pub timestamp: u32,
    pub tracks: Vec<XTrackItem>,
}

#[derive(Clone)]
pub struct YTPlaylist {
    pub title: String,
    pub url: String,
    pub videos: Vec<YoutubeVideo>,
}
