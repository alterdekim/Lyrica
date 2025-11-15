use crate::config::get_backup_itunesdb;
use crate::screens::search_util::SearchEntry;
use crate::sync::sync_util::{AppEvent, DBPlaylist, YTPlaylist};
use crate::util::IPodImage;
use crate::{
    config::{
        get_config_path, get_configs_dir, get_temp_dl_dir, get_temp_itunesdb, LyricaConfiguration,
    },
    util, AppState,
};
use id3::TagLike;
use image::imageops::FilterType;
use image::ImageReader;
use itunesdb::artworkdb::aobjects::ADatabase;
use itunesdb::objects::PlaylistItem;
use itunesdb::serializer;
use itunesdb::xobjects::{XDatabase, XPlArgument, XSomeList, XTrackItem};
use rand::random;
use soundcloud::sobjects::{CloudPlaylists, CloudTrack};
use std::io::Read;
use std::io::{Cursor, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
    sync::mpsc::{Sender, UnboundedReceiver},
};
use tokio_util::sync::CancellationToken;
use youtube_api::objects::YoutubeVideo;

mod audio_file_info;
mod downloader;
mod manager;
pub mod sync_util;

async fn track_from_video(
    value: &YoutubeVideo,
    ipod_path: String,
    sender: &Sender<AppEvent>,
) -> Option<XTrackItem> {
    let mut track_path = get_temp_dl_dir();
    track_path.push(&value.videoId);
    track_path.set_extension("mp3");
    let mut image_path = get_temp_dl_dir();
    image_path.push(&value.videoId);
    image_path.set_extension("webp");

    let audio_file = audio_file_info::from_path(track_path.to_str().unwrap())
        .await
        .unwrap();
    let audio_info = &audio_file.get_nice_object();
    let song_dbid = util::hash_from_path(track_path.clone());

    let size_in_bytes = File::open(track_path)
        .await
        .unwrap()
        .metadata()
        .await
        .unwrap()
        .size() as u32;

    let mut track = XTrackItem::new(
        random(),
        size_in_bytes,
        (audio_info.duration * 1000.0) as u32,
        0,
        (audio_info.bit_rate / 1000) as u32,
        audio_info.sample_rate as u32 * 0x10000,
        song_dbid,
        0,
    );

    track.data.mhii_link = size_in_bytes;

    if image_path.exists() {
        make_img(sender, ipod_path, song_dbid, image_path, &mut track).await;
    }

    audio_file.modify_xtrack(&mut track);

    track.set_title(value.title.clone());
    track.set_artist(value.publisher.clone());
    Some(track)
}

async fn make_img(
    sender: &Sender<AppEvent>,
    ipod_path: String,
    song_dbid: u64,
    image_path: PathBuf,
    track: &mut XTrackItem,
) {
    let _ = sender.send(AppEvent::ArtworkProgress((0, 2))).await;
    let mut adb = get_artwork_db(&ipod_path);

    let image_data = std::fs::read(image_path).unwrap();

    let cover_hash = util::hash(&image_data);

    let if_cover_present = adb.if_cover_present(cover_hash);

    let (small_img_name, large_img_name) = adb.add_images(song_dbid, cover_hash);

    let size = image_data.len();

    if !if_cover_present {
        make_cover_image(&image_data, &ipod_path, &small_img_name, (100, 100));
        let _ = sender.send(AppEvent::ArtworkProgress((1, 2))).await;
        make_cover_image(&image_data, &ipod_path, &large_img_name, (200, 200));
    }

    write_artwork_db(adb, &ipod_path);

    track.data.artwork_size = size as u32;
    track.data.has_artwork = 1;
    track.data.artwork_count = 1;
    let _ = sender.send(AppEvent::ArtworkProgress((2, 2))).await;
}

async fn track_from_soundcloud(
    value: &CloudTrack,
    ipod_path: String,
    sender: &Sender<AppEvent>,
) -> Option<XTrackItem> {
    let mut track_path = get_temp_dl_dir();
    track_path.push(value.id.to_string());
    track_path.set_extension("mp3");
    let mut image_path = get_temp_dl_dir();
    image_path.push(value.id.to_string());
    image_path.set_extension("jpg");
    let audio_file = audio_file_info::from_path(track_path.to_str().unwrap())
        .await
        .unwrap();
    let audio_info = &audio_file.get_nice_object();
    let song_dbid = util::hash_from_path(track_path.clone());

    let size_in_bytes = File::open(track_path)
        .await
        .unwrap()
        .metadata()
        .await
        .unwrap()
        .size() as u32;

    let mut track = XTrackItem::new(
        value.id as u32,
        size_in_bytes,
        (audio_info.duration * 1000.0) as u32,
        0,
        (audio_info.bit_rate / 1000) as u32,
        audio_info.sample_rate as u32,
        song_dbid,
        0,
    );

    track.data.mhii_link = size_in_bytes;

    if image_path.exists() {
        make_img(sender, ipod_path, song_dbid, image_path, &mut track).await;
    }

    audio_file.modify_xtrack(&mut track);

    track.set_title(value.title.clone().unwrap());
    track.set_artist(
        value
            .user
            .clone()
            .map_or(String::new(), |a| a.username.unwrap_or(a.permalink)),
    );
    if value.genre.is_some() {
        track.set_genre(value.genre.clone().unwrap());
    }
    Some(track)
}

fn get_track_location(unique_id: u32, extension: &str) -> String {
    let mut tp = PathBuf::new();
    tp.push(":iPod_Control");
    tp.push("Music");
    tp.push(["F", &format!("{:02}", &(unique_id % 100))].concat());
    tp.push(format!("{:X}", unique_id));
    tp.set_extension(extension);
    tp.to_str()
        .unwrap()
        .to_string()
        .replace("/", ":")
        .to_string()
}

fn get_full_track_location(p: PathBuf, unique_id: u32, extension: &str) -> PathBuf {
    let mut dest = p.clone();
    dest.push("iPod_Control");
    dest.push("Music");
    dest.push(["F", &format!("{:02}", &(unique_id % 100))].concat());
    let _ = std::fs::create_dir_all(dest.to_str().unwrap());
    dest.push(format!("{:X}", unique_id));
    dest.set_extension(extension);
    dest
}

fn get_itunesdb_location(path: &str) -> PathBuf {
    let mut p: PathBuf = Path::new(path).into();
    p.push("iPod_Control");
    p.push("iTunes");
    p.push("iTunesDB");
    p
}

fn overwrite_database(database: &mut XDatabase, ipod_path: &String) {
    let data = serializer::to_bytes(database);
    let p: PathBuf = get_itunesdb_location(ipod_path);

    let cd = get_backup_itunesdb();
    let _ = std::fs::copy(&p, &cd);

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
                            AppEvent::DownloadPlaylist(playlist) => { downloader::download_playlist(playlist, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await; },
                            AppEvent::DownloadTrack(track) => { downloader::download_track(track, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await; },
                            AppEvent::DownloadYTTrack(video) => { downloader::download_video(video, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await; },
                            AppEvent::DownloadYTPlaylist(ytplaylist) => { downloader::download_youtube_playlist(ytplaylist, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await; },
                            AppEvent::SwitchScreen(state) => { let _ = sender.send(AppEvent::SwitchScreen(state)).await;},
                            AppEvent::LoadFromFS(path) => {
                                let _ = sender
                                        .send(AppEvent::SwitchScreen(AppState::LoadingScreen))
                                        .await;
                                downloader::load_from_fs(path, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await;
                                    let _ = sender
                                        .send(AppEvent::SwitchScreen(AppState::FileSystem))
                                        .await;
                            },
                            AppEvent::LoadFromFSVec(files) => downloader::load_files_from_fs(files, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await,
                            AppEvent::LoadFromFSPL((files, title)) => downloader::load_files_from_fs_as_playlist(files, title, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await,
                            AppEvent::RemoveTrack(id) => manager::remove_track(id, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await,
                            AppEvent::RemovePlaylist((pl_id, is_hard)) => manager::remove_playlist(pl_id, is_hard, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await,
                            AppEvent::RemoveTrackFromPlaylist((track_id, pl_id)) => manager::remove_track_from_playlist(track_id, pl_id, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await,
                            AppEvent::SearchFor(query) => track_search(query, database.as_mut().unwrap(), &sender).await,
                            _ => {}
                        }
                    }
                }
            }
        }
    });
}

async fn track_search(query: String, database: &mut XDatabase, sender: &Sender<AppEvent>) {
    let mut results = Vec::new();

    let query = query.to_lowercase();

    if let XSomeList::TrackList(tracks) = &mut database.find_dataset(1).child {
        let mut tracks: Vec<SearchEntry> = tracks
            .iter()
            .filter(|i| {
                i.get_title().to_lowercase().contains(&query)
                    || i.get_artist().to_lowercase().contains(&query)
                    || i.get_album().to_lowercase().contains(&query)
                    || i.get_genre().to_lowercase().contains(&query)
            })
            .map(|i| {
                SearchEntry::track(
                    i.data.unique_id as u64,
                    i.get_title(),
                    i.get_artist(),
                    i.get_album(),
                    i.get_genre(),
                )
            })
            .collect();

        results.append(&mut tracks);
    }

    if let XSomeList::Playlists(playlists) = &mut database.find_dataset(3).child {
        let mut playlists = playlists
            .iter()
            .filter(|i| i.get_title().to_lowercase().contains(&query))
            .map(|i| SearchEntry::playlist(i.data.persistent_playlist_id, i.get_title()))
            .collect();

        results.append(&mut playlists);
    }

    let _ = sender.send(AppEvent::SearchShow(results)).await;
}

fn write_artwork_db(adb: ADatabase, ipod_path: &str) {
    let mut dst = PathBuf::from(ipod_path);
    dst.push("iPod_Control");
    dst.push("Artwork");
    dst.push("ArtworkDB");
    let bytes = itunesdb::artworkdb::serializer::to_bytes(adb);
    let mut f = std::fs::File::create(dst).unwrap();
    let _ = f.write(&bytes);
}

fn get_artwork_db(ipod_path: &str) -> ADatabase {
    let mut dst = PathBuf::from(ipod_path);
    dst.push("iPod_Control");
    dst.push("Artwork");
    dst.push("ArtworkDB");

    if dst.exists() {
        let mut f = std::fs::File::open(dst).unwrap();
        let mut buf = Vec::new();
        match f.read_to_end(&mut buf) {
            Ok(n) => {
                return itunesdb::artworkdb::deserializer::parse_bytes(&buf[..n]);
            }
            Err(_e) => {}
        }
    }
    itunesdb::artworkdb::deserializer::new_db()
}

fn make_cover_image(cover: &[u8], ipod_path: &str, file_name: &str, dim: (u32, u32)) {
    let mut dynamic_im = ImageReader::new(Cursor::new(cover))
        .with_guessed_format()
        .unwrap()
        .decode()
        .unwrap();

    if dynamic_im.height() != dynamic_im.width() {
        let side = if dynamic_im.height() < dynamic_im.width() {
            dynamic_im.height()
        } else {
            dynamic_im.width()
        };
        let x = if dynamic_im.height() < dynamic_im.width() {
            (dynamic_im.width() - side) / 2
        } else {
            0
        };
        let y = if dynamic_im.height() < dynamic_im.width() {
            0
        } else {
            (dynamic_im.height() - side) / 2
        };
        dynamic_im = dynamic_im.crop(x, y, side, side);
    }

    let img: IPodImage = dynamic_im
        .resize_exact(dim.0, dim.1, FilterType::Lanczos3)
        .into();

    let mut dst = PathBuf::from(ipod_path);
    dst.push("iPod_Control");
    dst.push("Artwork");

    let _ = std::fs::create_dir_all(dst.clone());

    dst.push(file_name);
    img.write(dst);
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
    let p = get_itunesdb_location(&path);
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

    let yt_sender = sender.clone();
    let yt_channel_id = config.get_youtube().user_id.clone();
    tokio::spawn(async move {
        let rid = youtube_api::get_channel(yt_channel_id.clone())
            .await
            .unwrap();
        let pls = youtube_api::get_playlists(yt_channel_id, rid)
            .await
            .unwrap();

        let mut yt_v = Vec::new();

        for pl in pls {
            let videos = youtube_api::get_playlist(pl.browse_id).await.unwrap();
            yt_v.push(YTPlaylist {
                title: pl.title,
                url: pl.pl_url,
                videos,
            });
        }

        let _ = yt_sender.send(AppEvent::YoutubeGot(yt_v)).await;
    });

    let soundcloud_user_id = config.get_soundcloud().user_id;
    let soundcloud_sender = sender.clone();
    tokio::spawn(async move {
        let app_version = soundcloud::get_app().await.unwrap().unwrap();
        let client_id = soundcloud::get_client_id().await.unwrap().unwrap();
        if let Ok(playlists) =
            soundcloud::get_playlists(soundcloud_user_id, client_id.clone(), app_version.clone())
                .await
        {
            let mut playlists = playlists.collection;

            for playlist in playlists.iter_mut() {
                let trr = playlist.tracks.clone();
                playlist.tracks = Vec::new();
                for pl_tracks in trr.clone().chunks(45) {
                    if let Ok(tracks) = soundcloud::get_tracks(
                        pl_tracks.to_vec(),
                        client_id.clone(),
                        app_version.clone(),
                    )
                    .await
                    {
                        let mut tracks = tracks;
                        tracks.retain(|t| t.title.is_some());
                        playlist.tracks.append(&mut tracks);
                    }
                }
            }

            let _ = soundcloud_sender
                .send(AppEvent::SoundcloudGot(CloudPlaylists {
                    collection: playlists,
                }))
                .await;
        }
    });

    database
}
