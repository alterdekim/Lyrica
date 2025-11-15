use crate::config::get_temp_dl_dir;
use crate::sync::sync_util::{AppEvent, YTPlaylist};
use crate::sync::{
    audio_file_info, get_artwork_db, get_full_track_location, get_playlists, get_track_location,
    make_cover_image, overwrite_database, track_from_soundcloud, track_from_video,
    write_artwork_db,
};
use crate::{dlp, util, AppState};
use audiotags::Tag;
use itunesdb::objects::ListSortOrder;
use itunesdb::xobjects::{XDatabase, XPlaylist, XTrackItem};
use ratatui::prelude::Color;
use soundcloud::sobjects::{CloudPlaylist, CloudTrack};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::sync::mpsc::Sender;
use youtube_api::objects::YoutubeVideo;

pub async fn download_playlist(
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

pub async fn download_track(
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

pub async fn download_video(
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

pub async fn download_youtube_playlist(
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

pub async fn load_from_fs(
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
    let audio_info = &audio_file.get_nice_object();

    let song_dbid = util::hash_from_path(path.clone());

    if !database.if_track_in_library(song_dbid) {
        let mut lyrics = 0;
        if let Ok(idmpeg) = id3::Tag::read_from_path(&path) {
            lyrics = idmpeg.lyrics().count();
        }

        let mut year = None;
        let mut title = None;
        let mut genre = None;
        let mut artist = None;
        let mut cover = None;
        let mut album = None;

        let mut track_number = None;
        let mut total_tracks = None;
        let mut disc_number = None;
        let mut total_discs = None;

        if let Ok(tag) = tag {
            year = tag.year();
            title = tag.title().map(|s| s.to_string());
            genre = tag.genre().map(|s| s.to_string());
            artist = tag.artist().map(|s| s.to_string());
            cover = tag.album_cover().map(|a| a.data.to_vec());
            album = tag.album_title().map(|a| a.to_string());

            track_number = tag.track_number();
            total_tracks = tag.total_tracks();
            disc_number = tag.disc_number();
            total_discs = tag.total_discs();
        }

        let size_in_bytes = File::open(path.clone())
            .await
            .unwrap()
            .metadata()
            .await
            .unwrap()
            .size() as u32;

        let mut track = XTrackItem::new(
            id,
            size_in_bytes,
            (audio_info.duration * 1000.0) as u32,
            year.unwrap_or(0) as u32,
            (audio_info.bit_rate / 1000) as u32,
            (audio_info.sample_rate * 65536) as u32,
            song_dbid,
            0,
        );

        audio_file.modify_xtrack(&mut track);

        track.data.gapless_album_flag = 1;
        track.data.gapless_track_flag = 0;

        track.data.mhii_link = size_in_bytes;

        track.data.lyrics_flag = (lyrics > 0) as u8;

        if let Some(track_number) = track_number {
            track.data.track_number = track_number as u32;
        }

        if let Some(total_tracks) = total_tracks {
            track.data.total_tracks = total_tracks as u32;
        }

        if let Some(disc_number) = disc_number {
            track.data.disc_number = disc_number as u32;
        }

        if let Some(total_discs) = total_discs {
            track.data.total_discs = total_discs as u32;
        }

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

pub async fn load_files_from_fs(
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

pub async fn load_files_from_fs_as_playlist(
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
