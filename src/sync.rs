use std::path::{Path, PathBuf};

use itunesdb::xobjects::XSomeList;
use redb::Database;
use soundcloud::sobjects::{CloudPlaylist, CloudPlaylists};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{Sender, UnboundedReceiver},
};
use tokio_util::sync::CancellationToken;

use crate::{config::{
    get_config_path, get_configs_dir, get_temp_dl_dir, get_temp_itunesdb, LyricaConfiguration,
}, db::{self, Track}, dlp::{self, DownloadProgress}, util, AppState};
use crate::db::{DBPlaylist, Playlist};

pub enum AppEvent {
    SearchIPod,
    IPodNotFound,
    ITunesParsed(Vec<DBPlaylist>),
    SoundcloudGot(CloudPlaylists),
    DownloadPlaylist(CloudPlaylist),
    CurrentProgress(DownloadProgress),
    OverallProgress((u32, u32)),
    SwitchScreen(AppState),
}

pub fn initialize_async_service(
    sender: Sender<AppEvent>,
    receiver: UnboundedReceiver<AppEvent>,
    token: CancellationToken,
) {
    tokio::spawn(async move {
        let _ = std::fs::create_dir_all(get_configs_dir());

        let mut ipod_db = None;

        let database = db::init_db();

        let mut receiver = receiver;

        loop {
            tokio::select! {
                _ = token.cancelled() => { return; }
                r = receiver.recv() => {
                    if let Some(request) = r {
                        match request {
                            AppEvent::SearchIPod => {
                                if let Some(p) = util::search_ipod() {
                                    let _ = sender.send(AppEvent::SwitchScreen(AppState::MainScreen)).await;
                                    ipod_db = Some(p.clone());
                                    parse_itunes(&database, &sender, p).await;
                                } else {
                                    let _ = sender.send(AppEvent::IPodNotFound).await;
                                }
                            },
                            AppEvent::DownloadPlaylist(playlist) => download_playlist(playlist, &database, &sender).await,
                            _ => {}
                        }
                    }
                }
            }
        }
    });
}

async fn download_playlist(
    playlist: CloudPlaylist,
    database: &Database,
    sender: &Sender<AppEvent>,
) {
    if let Ok(()) =
        dlp::download_from_soundcloud(&playlist.permalink_url, &get_temp_dl_dir(), sender.clone())
            .await
    {
        let tracks = playlist.tracks;
        for track in tracks {
            if track.title.is_none() {
                continue;
            }
            let mut t: Track = track.into();
            t.unique_id = db::get_last_track_id(database).unwrap_or(80) + 1;
            let _ = db::insert_track(database, t);
        }
    }
    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::MainScreen))
        .await;
}

async fn parse_itunes(database: &Database, sender: &Sender<AppEvent>, path: String) {
    let cd = get_temp_itunesdb();
    let mut p: PathBuf = Path::new(&path).into();
    p.push("iPod_Control");
    p.push("iTunes");
    p.set_file_name("iTunesDB");
    let _ = std::fs::copy(p, &cd);
    let mut file = File::open(cd).await.unwrap();
    let mut contents = vec![];
    file.read_to_end(&mut contents).await.unwrap();
    let mut xdb = itunesdb::deserializer::parse_bytes(&contents);

    if let XSomeList::TrackList(tracks) = &xdb.find_dataset(1).child {
        for track in tracks {
            let t: Track = track.clone().into();
            let _ = db::insert_track(database, t);
        }
    }

    if let XSomeList::Playlists(playlists) = &xdb.find_dataset(3).child {
        for playlist in playlists {
            let pl = Playlist {
                persistent_playlist_id: playlist.data.persistent_playlist_id,
                timestamp: playlist.data.timestamp,
                title: String::new() ,
                is_master: playlist.data.is_master_playlist_flag != 0,
                tracks: playlist.elems.iter().map(|e| e.0.track_id).collect()
            };
            let _ = db::insert_playlist(database, pl);
        }
    }

    let _ = sender
        .send(AppEvent::ITunesParsed(
            db::get_all_playlists(database).unwrap(),
        ))
        .await;

    let p = get_config_path();
    if !p.exists() {
        let config = LyricaConfiguration::default();
        let cfg_str = toml::to_string_pretty(&config).unwrap();
        let mut file = File::create(&p).await.unwrap();
        let _ = file.write(cfg_str.as_bytes()).await;
    }
    let mut file = File::open(p).await.unwrap();
    let mut content = String::new();
    file.read_to_string(&mut content).await.unwrap();
    let config: LyricaConfiguration = toml::from_str(&content).unwrap();

    let app_version = soundcloud::get_app().await.unwrap().unwrap();
    let client_id = soundcloud::get_client_id().await.unwrap().unwrap();
    let playlists = soundcloud::get_playlists(
        config.get_soundcloud().user_id,
        client_id.clone(),
        app_version.clone(),
    )
    .await
    .unwrap();

    let mut playlists = playlists.collection;

    for playlist in playlists.iter_mut() {
        if let Ok(tracks) = soundcloud::get_tracks(
            playlist.tracks.clone(),
            client_id.clone(),
            app_version.clone(),
        )
        .await
        {
            playlist.tracks = tracks;
        }
    }

    let _ = sender
        .send(AppEvent::SoundcloudGot(CloudPlaylists {
            collection: playlists,
        }))
        .await;
}
