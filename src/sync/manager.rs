use crate::sync::sync_util::AppEvent;
use crate::sync::{get_full_track_location, get_playlists, overwrite_database};
use crate::AppState;
use itunesdb::xobjects::XDatabase;
use ratatui::prelude::Color;
use std::path::PathBuf;
use tokio::sync::mpsc::Sender;

pub async fn remove_track(
    id: u32,
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String,
) {
    let _ = sender
        .send(AppEvent::OverallProgress((0, 1, Color::Red)))
        .await;
    database.remove_track_completely(id);
    for ext in ["mp3", "m4a", "wav", "aif"].iter() {
        let dest = get_full_track_location(PathBuf::from(ipod_path.clone()), id, ext);
        let _ = std::fs::remove_file(dest);
    }

    let _ = sender
        .send(AppEvent::OverallProgress((1, 1, Color::Red)))
        .await;

    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::MainScreen))
        .await;

    let _ = sender
        .send(AppEvent::ITunesParsed(get_playlists(database)))
        .await;

    overwrite_database(database, &ipod_path);
}

pub async fn remove_playlist(
    pl_id: u64,
    is_hard: bool,
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String,
) {
    if is_hard {
        let pls = database.get_playlists();
        let pl = pls.iter().find(|p| p.data.persistent_playlist_id == pl_id);
        if pl.is_none() {
            return;
        }
        let pl = pl.unwrap();
        let max = pl.elems.len();
        let mut i = 1;
        for (item, _args) in pl.elems.iter() {
            let _ = sender
                .send(AppEvent::OverallProgress((i, max as u32, Color::Red)))
                .await;
            remove_track(item.track_id, database, sender, ipod_path.clone()).await;
            i += 1;
        }
    }

    let _ = sender
        .send(AppEvent::OverallProgress((0, 1, Color::Red)))
        .await;

    database.remove_playlist(pl_id);

    let _ = sender
        .send(AppEvent::OverallProgress((1, 1, Color::Red)))
        .await;

    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::MainScreen))
        .await;

    let _ = sender
        .send(AppEvent::ITunesParsed(get_playlists(database)))
        .await;

    overwrite_database(database, &ipod_path);
}

pub async fn remove_track_from_playlist(
    track_id: u32,
    pl_id: u64,
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String,
) {
    let _ = sender
        .send(AppEvent::OverallProgress((0, 1, Color::Red)))
        .await;

    database.remove_track_from_playlist(track_id, pl_id);

    let _ = sender
        .send(AppEvent::OverallProgress((1, 1, Color::Red)))
        .await;

    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::MainScreen))
        .await;

    let _ = sender
        .send(AppEvent::ITunesParsed(get_playlists(database)))
        .await;

    overwrite_database(database, &ipod_path);
}
