use std::path::{Path, PathBuf};

use itunesdb::xobjects::{XDatabase, XTrackItem};
use soundcloud::sobjects::{CloudPlaylist, CloudPlaylists};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{Sender, UnboundedReceiver},
};
use tokio_util::sync::CancellationToken;

use crate::{config::{
    get_config_path, get_configs_dir, get_temp_dl_dir, get_temp_itunesdb, LyricaConfiguration,
}, dlp::{self, DownloadProgress}, util, AppState};

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

pub struct DBPlaylist {
    pub id: u64,
    pub title: String,
    pub timestamp: u32,
    pub tracks: Vec<XTrackItem>
}

pub fn initialize_async_service(
    sender: Sender<AppEvent>,
    receiver: UnboundedReceiver<AppEvent>,
    token: CancellationToken,
) {
    tokio::spawn(async move {
        let _ = std::fs::create_dir_all(get_configs_dir());

        let mut ipod_db = None;

        let mut database = None;

        let mut receiver = receiver;

        loop {
            tokio::select! {
                _ = token.cancelled() => { return; }
                r = receiver.recv() => {
                    if let Some(request) = r {
                        match request {
                            AppEvent::SearchIPod => {
                                if let Some(p) = util::search_ipod() {
                                    ipod_db = Some(p.clone());
                                    database = Some(parse_itunes(&sender, p).await);
                                    let _ = sender.send(AppEvent::SwitchScreen(AppState::MainScreen)).await;
                                } else {
                                    let _ = sender.send(AppEvent::IPodNotFound).await;
                                }
                            },
                            AppEvent::DownloadPlaylist(playlist) => download_playlist(playlist, &mut database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await,
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
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String
) {
    if let Ok(()) =
        dlp::download_from_soundcloud(&playlist.permalink_url, &get_temp_dl_dir(), sender.clone())
            .await
    {
        let tracks = playlist.tracks;

        let mut p: PathBuf = Path::new(&ipod_path).into();
        for track in tracks {
            if track.title.is_none() {
                continue;
            }
            let mut t: XTrackItem = track.into();
            t.data.unique_id = database.get_unique_id();
            let mut tp = PathBuf::new();
            tp.push("iPod_Control");
            tp.push("Music");
            tp.push(["F", &format!("{:02}", &(t.data.unique_id % 100))].concat());
            tp.push(format!("{:X}", t.data.unique_id));
            tp.set_extension("mp3");
            t.set_location(tp.to_str().unwrap().to_string().replace("/", ":").to_string());
            let mut dest = p.clone();
            dest.push(tp);

            let mut track_path = get_temp_dl_dir();
            track_path.push(track.id.to_string());
            track_path.set_extension("mp3");

            let _ = std::fs::copy(track_path, dest);

            let _ = database.add_track(t);
        }
    }
    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::MainScreen))
        .await;
}

fn get_playlists(db: &mut XDatabase) -> Vec<DBPlaylist> {
    let pls = db.get_playlists(); // string arg type 1 - playlist title.
    pls.iter()
        .map(|t| DBPlaylist {
            id: t.data.persistent_playlist_id,
            title: t.get_title(),
            timestamp: t.data.timestamp,
            tracks: t.elems.iter().map(|(i, _a)| db.get_track(i.track_id)).filter(|t| t.is_some()).map(|t| t.unwrap().clone()).collect()}).collect()
}

async fn parse_itunes(sender: &Sender<AppEvent>, path: String) -> XDatabase {
    let cd = get_temp_itunesdb();
    let mut p: PathBuf = Path::new(&path).into();
    p.push("iPod_Control");
    p.push("iTunes");
    p.push("iTunesDB");
    println!("{}", p.to_str().unwrap());
    let _ = std::fs::copy(p, &cd);
    let mut file = File::open(cd).await.unwrap();
    let mut contents = vec![];
    file.read_to_end(&mut contents).await.unwrap();
    let mut database = itunesdb::deserializer::parse_bytes(&contents);

    let _ = sender
        .send(AppEvent::ITunesParsed(
            get_playlists(&mut database),
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

    database
}
