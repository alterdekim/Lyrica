use crate::util::IPodImage;
use crate::{
    config::{
        get_config_path, get_configs_dir, get_temp_dl_dir, get_temp_itunesdb, LyricaConfiguration,
    },
    dlp::{self, DownloadProgress},
    util, AppState,
};
use audiotags::Tag;
use image::imageops::FilterType;
use image::ImageReader;
use itunesdb::artworkdb::aobjects::ADatabase;
use itunesdb::objects::{ListSortOrder, PlaylistItem};
use itunesdb::serializer;
use itunesdb::xobjects::{XDatabase, XPlArgument, XPlaylist, XTrackItem};
use rand::random;
use ratatui::style::Color;
use soundcloud::sobjects::{CloudPlaylist, CloudPlaylists, CloudTrack};
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
    let audio_info = &audio_file.audio_file.tracks.track;
    let song_dbid = util::hash_from_path(track_path.clone());

    let mut track = XTrackItem::new(
        random(),
        File::open(track_path)
            .await
            .unwrap()
            .metadata()
            .await
            .unwrap()
            .size() as u32,
        (audio_info.duration * 1000.0) as u32,
        0,
        (audio_info.bit_rate / 1000) as u32,
        audio_info.sample_rate as u32,
        song_dbid,
        0,
    );

    if image_path.exists() {
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
        track.data.mhii_link = 0;
        track.data.has_artwork = 1;
        track.data.artwork_count = 1;
        let _ = sender.send(AppEvent::ArtworkProgress((2, 2))).await;
    }

    audio_file.modify_xtrack(&mut track);

    track.set_title(value.title.clone());
    track.set_artist(value.publisher.clone());
    Some(track)
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
    let audio_info = &audio_file.audio_file.tracks.track;
    let song_dbid = util::hash_from_path(track_path.clone());

    let mut track = XTrackItem::new(
        value.id as u32,
        File::open(track_path)
            .await
            .unwrap()
            .metadata()
            .await
            .unwrap()
            .size() as u32,
        (audio_info.duration * 1000.0) as u32,
        0,
        (audio_info.bit_rate / 1000) as u32,
        audio_info.sample_rate as u32,
        song_dbid,
        0,
    );

    if image_path.exists() {
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
        track.data.mhii_link = 0;
        track.data.has_artwork = 1;
        track.data.artwork_count = 1;
        let _ = sender.send(AppEvent::ArtworkProgress((2, 2))).await;
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
                            AppEvent::DownloadYTTrack(video) => { download_video(video, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await; },
                            AppEvent::DownloadYTPlaylist(ytplaylist) => { download_youtube_playlist(ytplaylist, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await; },
                            AppEvent::SwitchScreen(state) => { let _ = sender.send(AppEvent::SwitchScreen(state)).await;},
                            AppEvent::LoadFromFS(path) => {
                                let _ = sender
                                        .send(AppEvent::SwitchScreen(AppState::LoadingScreen))
                                        .await;
                                load_from_fs(path, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await;
                                    let _ = sender
                                        .send(AppEvent::SwitchScreen(AppState::FileSystem))
                                        .await;
                            },
                            AppEvent::LoadFromFSVec(files) => load_files_from_fs(files, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await,
                            AppEvent::LoadFromFSPL((files, title)) => load_files_from_fs_as_playlist(files, title, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await,
                            AppEvent::RemoveTrack(id) => remove_track(id, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await,
                            AppEvent::RemovePlaylist((pl_id, is_hard)) => remove_playlist(pl_id, is_hard, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await,
                            AppEvent::RemoveTrackFromPlaylist((track_id, pl_id)) => remove_track_from_playlist(track_id, pl_id, database.as_mut().unwrap(), &sender, ipod_db.clone().unwrap()).await,
                            _ => {}
                        }
                    }
                }
            }
        }
    });
}

async fn remove_track_from_playlist(
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

async fn remove_playlist(
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

async fn remove_track(
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

async fn load_files_from_fs_as_playlist(
    files: Vec<PathBuf>,
    title: String,
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String,
) {
    let mut new_playlist = XPlaylist::new(rand::random(), ListSortOrder::SongTitle);

    new_playlist.set_title(title);

    for (i, file) in files.iter().enumerate() {
        let _ = sender
            .send(AppEvent::OverallProgress((
                i as u32,
                files.len() as u32,
                Color::Green,
            )))
            .await;
        let id = load_from_fs(file.clone(), database, sender, ipod_path.clone()).await;

        new_playlist.add_elem(id);
    }

    database.add_playlist(new_playlist);

    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::FileSystem))
        .await;

    let _ = sender
        .send(AppEvent::ITunesParsed(get_playlists(database)))
        .await;

    overwrite_database(database, &ipod_path);
}

async fn load_files_from_fs(
    files: Vec<PathBuf>,
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String,
) {
    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::LoadingScreen))
        .await;
    for (i, file) in files.iter().enumerate() {
        let _ = sender
            .send(AppEvent::OverallProgress((
                i as u32,
                files.len() as u32,
                Color::Green,
            )))
            .await;
        load_from_fs(file.clone(), database, sender, ipod_path.clone()).await;
    }

    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::FileSystem))
        .await;
}

async fn load_from_fs(
    path: PathBuf,
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String,
) -> u32 {
    let tag = Tag::new().read_from_path(&path);

    let mut id = database.get_unique_id();

    let audio_file = audio_file_info::from_path(path.to_str().unwrap())
        .await
        .unwrap();
    let audio_info = &audio_file.audio_file.tracks.track;

    let song_dbid = util::hash_from_path(path.clone());

    if !database.if_track_in_library(song_dbid) {
        let mut year = None;
        let mut title = None;
        let mut genre = None;
        let mut artist = None;
        let mut cover = None;
        let mut album = None;

        if let Ok(tag) = tag {
            year = tag.year();
            title = tag.title().map_or(None, |s| Some(s.to_string()));
            genre = tag.genre().map_or(None, |s| Some(s.to_string()));
            artist = tag.artist().map_or(None, |s| Some(s.to_string()));
            cover = tag.album_cover().map_or(None, |a| Some(a.data.to_vec()));
            album = tag.album_title().map_or(None, |a| Some(a.to_string()));
        }

        let mut track = XTrackItem::new(
            id,
            File::open(path.clone())
                .await
                .unwrap()
                .metadata()
                .await
                .unwrap()
                .size() as u32,
            (audio_info.duration * 1000.0) as u32,
            year.unwrap_or(0) as u32,
            (audio_info.bit_rate / 1000) as u32,
            audio_info.sample_rate as u32,
            song_dbid,
            0,
        );

        audio_file.modify_xtrack(&mut track);

        if let Some(title) = title {
            track.set_title(title.to_string());
        } else {
            track.set_title(path.file_name().unwrap().to_str().unwrap().to_string());
        }

        if let Some(genre) = genre {
            track.set_genre(genre.to_string());
        }

        if let Some(artist) = artist {
            track.set_artist(artist.to_string());
        }

        if let Some(cover) = cover {
            let _ = sender.send(AppEvent::ArtworkProgress((0, 2))).await;

            let mut adb = get_artwork_db(&ipod_path);

            let cover_hash = util::hash(&cover);

            let if_cover_present = adb.if_cover_present(cover_hash);

            let (small_img_name, large_img_name) = adb.add_images(song_dbid, cover_hash);

            let size = cover.len();

            if !if_cover_present {
                make_cover_image(&cover, &ipod_path, &small_img_name, (100, 100));
                let _ = sender.send(AppEvent::ArtworkProgress((1, 2))).await;
                make_cover_image(&cover, &ipod_path, &large_img_name, (200, 200));
            }

            write_artwork_db(adb, &ipod_path);

            track.data.artwork_size = size as u32;
            track.data.mhii_link = 0;
            track.data.has_artwork = 1;
            track.data.artwork_count = 1;

            let _ = sender.send(AppEvent::ArtworkProgress((2, 2))).await;
        }

        if let Some(album) = album {
            track.set_album(album);
            // TODO: Add new album into iTunesDB
        }

        track.set_location(get_track_location(
            track.data.unique_id,
            audio_file.get_audio_extension(),
        ));

        let dest = get_full_track_location(
            PathBuf::from(ipod_path.clone()),
            track.data.unique_id,
            audio_file.get_audio_extension(),
        );

        let _ = std::fs::copy(path.to_str().unwrap(), dest.to_str().unwrap());

        database.add_track(track);

        overwrite_database(database, &ipod_path);
    } else if let Some(unique_id) = database.get_unique_id_by_dbid(song_dbid) {
        id = unique_id;
    }

    let _ = sender
        .send(AppEvent::ITunesParsed(get_playlists(database)))
        .await;

    id
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

async fn download_video(
    video: YoutubeVideo,
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String,
) {
    if let Ok(()) =
        dlp::download_track_from_youtube(&video.videoId.clone(), &get_temp_dl_dir(), sender.clone())
            .await
    {
        let p: PathBuf = Path::new(&ipod_path).into();

        if let Some(mut t) = track_from_video(&video, ipod_path.clone(), sender).await {
            if !database.if_track_in_library(t.data.dbid) {
                t.data.unique_id = database.get_unique_id();
                t.set_location(get_track_location(t.data.unique_id, "mp3"));
                let dest = get_full_track_location(p.clone(), t.data.unique_id, "mp3");

                let mut track_path = get_temp_dl_dir();
                track_path.push(&video.videoId);
                track_path.set_extension("mp3");

                let _ = std::fs::copy(track_path.to_str().unwrap(), dest.to_str().unwrap());

                database.add_track(t);
            }
        }
    }

    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::MainScreen))
        .await;

    let _ = sender
        .send(AppEvent::ITunesParsed(get_playlists(database)))
        .await;

    overwrite_database(database, &ipod_path);

    crate::config::clear_temp_dl_dir();
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

        if let Some(mut t) = track_from_soundcloud(&track, ipod_path.clone(), sender).await {
            if !database.if_track_in_library(t.data.dbid) {
                t.data.unique_id = database.get_unique_id();
                t.set_location(get_track_location(t.data.unique_id, "mp3"));
                let dest = get_full_track_location(p.clone(), t.data.unique_id, "mp3");

                let mut track_path = get_temp_dl_dir();
                track_path.push(track.id.to_string());
                track_path.set_extension("mp3");

                let _ = std::fs::copy(track_path.to_str().unwrap(), dest.to_str().unwrap());

                database.add_track(t);
            }
        }
    }

    let _ = sender
        .send(AppEvent::SwitchScreen(AppState::MainScreen))
        .await;

    let _ = sender
        .send(AppEvent::ITunesParsed(get_playlists(database)))
        .await;

    overwrite_database(database, &ipod_path);

    crate::config::clear_temp_dl_dir();
}

async fn download_youtube_playlist(
    playlist: YTPlaylist,
    database: &mut XDatabase,
    sender: &Sender<AppEvent>,
    ipod_path: String,
) {
    if let Ok(()) =
        dlp::download_from_youtube(&playlist.url, &get_temp_dl_dir(), sender.clone()).await
    {
        let videos = playlist.videos;

        let p: PathBuf = Path::new(&ipod_path).into();

        let mut new_playlist = XPlaylist::new(rand::random(), ListSortOrder::SongTitle);

        new_playlist.set_title(playlist.title);

        for video in videos {
            if let Some(mut t) = track_from_video(&video, ipod_path.clone(), sender).await {
                if !database.if_track_in_library(t.data.dbid) {
                    t.data.unique_id = database.get_unique_id();
                    new_playlist.add_elem(t.data.unique_id);
                    t.set_location(get_track_location(t.data.unique_id, "mp3"));
                    let dest = get_full_track_location(p.clone(), t.data.unique_id, "mp3");

                    let mut track_path = get_temp_dl_dir();
                    track_path.push(&video.videoId);
                    track_path.set_extension("mp3");

                    let _ = std::fs::copy(track_path.to_str().unwrap(), dest.to_str().unwrap());

                    database.add_track(t);
                } else if let Some(unique_id) = database.get_unique_id_by_dbid(t.data.dbid) {
                    new_playlist.add_elem(unique_id);
                }
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

    crate::config::clear_temp_dl_dir();
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
            if let Some(mut t) = track_from_soundcloud(&track, ipod_path.clone(), sender).await {
                if !database.if_track_in_library(t.data.dbid) {
                    t.data.unique_id = database.get_unique_id();
                    new_playlist.add_elem(t.data.unique_id);
                    t.set_location(get_track_location(t.data.unique_id, "mp3"));
                    let dest = get_full_track_location(p.clone(), t.data.unique_id, "mp3");
                    let mut track_path = get_temp_dl_dir();
                    track_path.push(track.id.to_string());
                    track_path.set_extension("mp3");

                    let _ = std::fs::copy(track_path.to_str().unwrap(), dest.to_str().unwrap());
                    database.add_track(t);
                } else if let Some(unique_id) = database.get_unique_id_by_dbid(t.data.dbid) {
                    new_playlist.add_elem(unique_id);
                }
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

    crate::config::clear_temp_dl_dir();
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

mod audio_file_info {
    use itunesdb::xobjects::XTrackItem;
    use serde::Deserialize;
    use std::process::Stdio;
    use tokio::io::{AsyncReadExt, BufReader};
    use tokio::process::Command;

    #[derive(Debug, Deserialize, PartialEq)]
    pub struct AudioInfo {
        pub audio_file: AudioFileInfo,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    pub struct AudioFileInfo {
        pub file_name: String,
        pub file_type: String,
        pub tracks: AudioFileTracks,
    }

    impl AudioInfo {
        pub fn get_audio_extension(&self) -> &str {
            match self.audio_file.file_type.as_str() {
                "'WAVE'" => "wav",
                "'AIFF'" => "aif",
                "'m4af'" => "m4a",
                _ => "mp3",
            }
        }

        fn get_audio_codec(&self) -> String {
            match self.audio_file.file_type.as_str() {
                "'WAVE'" => "WAV audio file",
                "'AIFF'" => "AIFF audio file",
                "'m4af'" => match self.audio_file.tracks.track.format_type.as_str() {
                    "alac" => "Apple Lossless audio file",
                    _ => "AAC audio file",
                },
                _ => "MPEG audio file",
            }
            .to_string()
        }

        pub fn modify_xtrack(&self, track: &mut XTrackItem) {
            track.data.type1 = 0;
            track.data.type2 = if self.audio_file.file_type == "'MPG3'" {
                1
            } else {
                0
            };

            let bytes = match self.audio_file.file_type.as_str() {
                "'WAVE'" => "WAV",
                "'AIFF'" => "AIF",
                "'m4af'" => match self.audio_file.tracks.track.format_type.as_str() {
                    "alac" => "M4A ",
                    _ => "M4A",
                },
                _ => "MP3",
            }
            .as_bytes();

            let file_type = u32::from_be_bytes(if bytes.len() == 4 {
                [bytes[0], bytes[1], bytes[2], bytes[3]]
            } else {
                [bytes[0], bytes[1], bytes[2], 0u8]
            });

            track.data.filetype = file_type;

            track.update_arg(6, self.get_audio_codec());
        }
    }

    #[derive(Deserialize, Debug, PartialEq)]
    pub struct AudioFileTracks {
        pub track: AudioFileTrack,
    }

    #[derive(Debug, Deserialize, PartialEq)]
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
        command.arg("-x");
        command.arg(p);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::null());

        let mut child = command.spawn().unwrap();

        let mut vec = Vec::new();
        let stdout = child.stdout.take().unwrap();
        let size = BufReader::new(stdout)
            .read_to_end(&mut vec)
            .await
            .unwrap_or(0);
        if size == 0 {
            return None;
        }

        /*let mut f = File::create("afinfo_out.xml").unwrap();
        let _ = f.write(str.as_bytes());*/
        Some(serde_xml_rs::from_str(String::from_utf8_lossy(vec.as_slice()).as_ref()).unwrap())
    }
}
