use audiotags::Tag;
use itunesdb::objects::{ListSortOrder, PlaylistItem};
use itunesdb::serializer;
use itunesdb::xobjects::{XDatabase, XPlArgument, XPlaylist, XTrackItem};
use soundcloud::sobjects::{CloudPlaylist, CloudPlaylists, CloudTrack};
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{Sender, UnboundedReceiver},
};
use tokio_util::sync::CancellationToken;

use crate::{
    config::{
        get_config_path, get_configs_dir, get_temp_dl_dir, get_temp_itunesdb, LyricaConfiguration,
    },
    dlp::{self, DownloadProgress},
    util, AppState,
};

pub enum AppEvent {
    SearchIPod,
    IPodNotFound,
    ITunesParsed(Vec<DBPlaylist>),
    SoundcloudGot(CloudPlaylists),
    DownloadPlaylist(CloudPlaylist),
    DownloadTrack(CloudTrack),
    CurrentProgress(DownloadProgress),
    OverallProgress((u32, u32)),
    SwitchScreen(AppState),
    LoadFromFS(PathBuf),
}

pub struct DBPlaylist {
    pub id: u64,
    pub title: String,
    pub timestamp: u32,
    pub tracks: Vec<XTrackItem>,
}

async fn track_from_soundcloud(value: &CloudTrack) -> Option<XTrackItem> {
    let mut track_path = get_temp_dl_dir();
    track_path.push(value.id.to_string());
    track_path.set_extension("mp3");
    let f = std::fs::File::open(&track_path).unwrap();
    let mut data = &std::fs::read(&track_path).unwrap()[..];
    let audio_info = audio_file_info::from_path(track_path.to_str().unwrap())
        .await
        .unwrap();
    let audio_info = audio_info.audio_file.tracks.get(0).unwrap();

    let mut track = XTrackItem::new(
        value.id as u32,
        f.metadata().unwrap().len() as u32,
        (audio_info.duration * 1000.0) as u32,
        0,
        audio_info.bit_rate as u32,
        audio_info.sample_rate as u32,
        hash(),
        0,
    );
    track.set_title(value.title.clone().unwrap());
    track.set_artist(
        value
            .user
            .clone()
            .map_or(String::new(), |a| a.username.unwrap_or(a.permalink)),
    );
    track.set_genre(value.genre.clone().unwrap());
    track.update_arg(6, String::from("MPEG audio file"));
    Some(track)
}

// note: this hash function is used to make unique ids for each track. It doesn't aim to generate secure ones.
fn hash() -> u64 {
    rand::random::<u64>()
}

fn overwrite_database(database: &mut XDatabase, ipod_path: &String) {
    let data = serializer::to_bytes(database);
    let mut p: PathBuf = Path::new(ipod_path).into();
    p.push("iPod_Control");
    p.push("iTunes");
    p.push("iTunesDB");
    let mut file = std::fs::File::create(p).unwrap();
    let _ = file.write(&data);
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
                                    let _ = sender.send(AppEvent::SwitchScreen(AppState::MainScreen)).await;
                                    database = Some(parse_itunes(&sender, p).await);
                                } else {
                                    let _ = sender.send(AppEvent::IPodNotFound).await;
                                }
                            },
                            AppEvent::DownloadPlaylist(playlist) => { download_playlist(playlist, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await; },
                            AppEvent::DownloadTrack(track) => { download_track(track, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await; },
                            AppEvent::SwitchScreen(state) => { let _ = sender.send(AppEvent::SwitchScreen(state)).await;},
                            AppEvent::LoadFromFS(path) => load_from_fs(path, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()),
                            _ => {}
                        }
                    }
                }
            }
        }
    });
}

fn load_from_fs(
    path: PathBuf,
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String,
) {
    let mut tag = Tag::new().read_from_path(path).unwrap();

    let id = database.get_unique_id();

    let mut track = XTrackItem::new(
        id,
        0,
        (tag.duration().unwrap() / 1000.0) as u32,
        tag.year().unwrap() as u32,
        0,
        0,
        hash(),
        0,
    );
    // TODO: implement check for every property
    track.set_title(tag.title().unwrap().to_string());

    let mut tp = PathBuf::new();
    tp.push(":iPod_Control");
    tp.push("Music");
    tp.push(["F", &format!("{:02}", &(track.data.unique_id % 100))].concat());
    tp.push(format!("{:X}", track.data.unique_id));
    tp.set_extension("mp3");
    track.set_location(
        tp.to_str()
            .unwrap()
            .to_string()
            .replace("/", ":")
            .to_string(),
    );
    //track.update_arg(6);

    track.set_genre(tag.genre().unwrap().to_string());
    track.set_artist(tag.artist().unwrap().to_string());
    track.set_album(tag.album().unwrap().title.to_string());

    let cover = tag.album().unwrap().cover.unwrap();
}

async fn download_track(
    track: CloudTrack,
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String,
) {
    if let Ok(()) = dlp::download_track_from_soundcloud(
        &track.permalink_url.clone().unwrap(),
        &get_temp_dl_dir(),
        sender.clone(),
    )
    .await
    {
        let p: PathBuf = Path::new(&ipod_path).into();

        if let Some(mut t) = track_from_soundcloud(&track).await {
            t.data.unique_id = database.get_unique_id();
            let mut tp = PathBuf::new();
            tp.push(":iPod_Control");
            tp.push("Music");
            tp.push(["F", &format!("{:02}", &(t.data.unique_id % 100))].concat());
            tp.push(format!("{:X}", t.data.unique_id));
            tp.set_extension("mp3");
            t.set_location(
                tp.to_str()
                    .unwrap()
                    .to_string()
                    .replace("/", ":")
                    .to_string(),
            );
            let mut dest = p.clone();
            dest.push("iPod_Control");
            dest.push("Music");
            dest.push(["F", &format!("{:02}", &(t.data.unique_id % 100))].concat());
            let _ = std::fs::create_dir_all(dest.to_str().unwrap());
            dest.push(format!("{:X}", t.data.unique_id));
            dest.set_extension("mp3");

            let mut track_path = get_temp_dl_dir();
            track_path.push(track.id.to_string());
            track_path.set_extension("mp3");

            let _ = std::fs::copy(track_path.to_str().unwrap(), dest.to_str().unwrap());

            database.add_track(t);
        }
    }

    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::MainScreen))
        .await;

    let _ = sender
        .send(AppEvent::ITunesParsed(get_playlists(database)))
        .await;

    overwrite_database(database, &ipod_path);
}

async fn download_playlist(
    playlist: CloudPlaylist,
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String,
) {
    if let Ok(()) =
        dlp::download_from_soundcloud(&playlist.permalink_url, &get_temp_dl_dir(), sender.clone())
            .await
    {
        let tracks = playlist.tracks;

        let p: PathBuf = Path::new(&ipod_path).into();

        let mut new_playlist = XPlaylist::new(rand::random(), ListSortOrder::SongTitle);

        new_playlist.set_title(playlist.title);

        for track in tracks {
            if track.title.is_none() {
                continue;
            }
            if let Some(mut t) = track_from_soundcloud(&track).await {
                t.data.unique_id = database.get_unique_id();
                new_playlist.add_elem(t.data.unique_id);
                let mut tp = PathBuf::new();
                tp.push(":iPod_Control");
                tp.push("Music");
                tp.push(["F", &format!("{:02}", &(t.data.unique_id % 100))].concat());
                tp.push(format!("{:X}", t.data.unique_id));
                tp.set_extension("mp3");
                t.set_location(
                    tp.to_str()
                        .unwrap()
                        .to_string()
                        .replace("/", ":")
                        .to_string(),
                );
                let mut dest = p.clone();
                dest.push("iPod_Control");
                dest.push("Music");
                dest.push(["F", &format!("{:02}", &(t.data.unique_id % 100))].concat());
                let _ = std::fs::create_dir_all(dest.to_str().unwrap());
                dest.push(format!("{:X}", t.data.unique_id));
                dest.set_extension("mp3");

                let mut track_path = get_temp_dl_dir();
                track_path.push(track.id.to_string());
                track_path.set_extension("mp3");

                let _ = std::fs::copy(track_path.to_str().unwrap(), dest.to_str().unwrap());

                database.add_track(t);
            }
        }

        database.add_playlist(new_playlist);
    }

    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::MainScreen))
        .await;

    let _ = sender
        .send(AppEvent::ITunesParsed(get_playlists(database)))
        .await;

    overwrite_database(database, &ipod_path);
}

fn get_playlists(db: &mut XDatabase) -> Vec<DBPlaylist> {
    let pls = db.get_playlists(); // string arg type 1 - playlist title.
    pls.iter()
        .map(|t| DBPlaylist {
            id: t.data.persistent_playlist_id,
            title: t.get_title(),
            timestamp: t.data.timestamp,
            tracks: to_tracks(db, t.elems.clone()),
        })
        .collect()
}

fn to_tracks(db: &mut XDatabase, elems: Vec<(PlaylistItem, Vec<XPlArgument>)>) -> Vec<XTrackItem> {
    elems
        .iter()
        .map(|(i, _a)| i.track_id)
        .map(|id| db.get_track(id))
        .filter(|i| i.is_some())
        .map(|i| i.unwrap().clone())
        .collect()
}

async fn parse_itunes(sender: &Sender<AppEvent>, path: String) -> XDatabase {
    let cd = get_temp_itunesdb();
    let mut p: PathBuf = Path::new(&path).into();
    p.push("iPod_Control");
    p.push("iTunes");
    p.push("iTunesDB");
    let _ = std::fs::copy(p, &cd);
    let mut file = File::open(cd).await.unwrap();
    let mut contents = vec![];
    file.read_to_end(&mut contents).await.unwrap();
    let mut database = itunesdb::deserializer::parse_bytes(&contents);

    let _ = sender
        .send(AppEvent::ITunesParsed(get_playlists(&mut database)))
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

mod audio_file_info {
    use serde::{Deserialize, Serialize};
    use std::process::Stdio;
    use tokio::io::{AsyncReadExt, BufReader};
    use tokio::process::Command;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct AudioInfo {
        pub audio_file: AudioFileInfo,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct AudioFileInfo {
        pub file_name: String,
        pub file_type: String,
        pub tracks: Vec<AudioFileTrack>,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    pub struct AudioFileTrack {
        pub num_channels: u32,
        pub sample_rate: u64,
        pub format_type: String,
        pub audio_bytes: u64,
        pub duration: f64,
        pub bit_rate: u64,
    }

    pub async fn from_path(p: &str) -> Option<AudioInfo> {
        let mut command = Command::new("afinfo");
        command.args(vec!["-x", p]);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::null());

        let mut child = command.spawn().unwrap();

        let mut str = String::new();
        let stdout = child.stdout.take().unwrap();
        let size = BufReader::new(stdout)
            .read_to_string(&mut str)
            .await
            .unwrap();
        Some(serde_xml_rs::from_str(&str).unwrap())
    }
}
