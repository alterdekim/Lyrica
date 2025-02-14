use std::fs::File;
use std::ops::Deref;
use itunesdb::xobjects::{XArgument, XPlaylist, XTrackItem};
use md5::{Digest, Md5};
use redb::{Database, Error, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use soundcloud::sobjects::CloudTrack;

use crate::config::{get_db, get_temp_dl_dir};

const TRACKS: TableDefinition<u32, Vec<u8>> = TableDefinition::new("tracks");
const PLAYLISTS: TableDefinition<u64, Vec<u8>> = TableDefinition::new("playlists");

#[derive(Serialize, Deserialize)]
pub struct Track {
    pub unique_id: u32,
    filetype: u32,
    stars: u8,
    last_modified_time: u32,
    size: u32,
    pub length: u32,
    year: u32,
    pub bitrate: u32,
    sample_rate: u32,
    play_count: u32,
    pub dbid: u64,
    bpm: u16,
    skip_count: u32,
    has_artwork: u8,
    media_type: u32,
    pub title: String,
    pub location: String,
    album: String,
    pub artist: String,
    pub genre: String,
}

#[derive(Serialize, Deserialize)]
pub struct DBPlaylist {
    pub persistent_playlist_id: u64,
    pub title: String,
    pub timestamp: u32,
    pub is_master: bool,
    pub tracks: Vec<Track>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Playlist {
    pub persistent_playlist_id: u64,
    pub title: String,
    pub timestamp: u32,
    pub is_master: bool,
    pub tracks: Vec<u32>,
}

impl From<CloudTrack> for Track {
    fn from(value: CloudTrack) -> Self {
        let mut track_path = get_temp_dl_dir();
        track_path.push(value.id.to_string());
        track_path.set_extension("mp3");
        let f = File::open(&track_path).unwrap();
        let data = &std::fs::read(&track_path).unwrap()[..];
        let (header, _samples) = puremp3::read_mp3(data).unwrap();
        Track {
            unique_id: 0,
            filetype: 0,
            stars: 0,
            last_modified_time: 0,
            size: f.metadata().unwrap().len() as u32,
            length: 0,
            year: 0,
            bitrate: header.bitrate.bps() / 1000,
            sample_rate: header.sample_rate.hz(),
            play_count: 0,
            dbid: hash(data),
            bpm: 0,
            skip_count: 0,
            has_artwork: 0,
            media_type: 0,
            title: value.title.unwrap(),
            location: String::new(),
            album: String::new(),
            artist: value
                .user
                .map_or(String::new(), |a| a.username.unwrap_or(a.permalink)),
            genre: value.genre.unwrap_or_default(),
        }
    }
}

fn find_str_arg(value: &XTrackItem, arg_type: u32) -> Option<&XArgument> {
    value.args.iter().find(|arg| arg.arg_type == arg_type)
}

impl From<XTrackItem> for Track {
    fn from(value: XTrackItem) -> Self {
        Track {
            unique_id: value.data.unique_id,
            filetype: value.data.filetype,
            stars: value.data.stars,
            last_modified_time: value.data.last_modified_time,
            size: value.data.size,
            length: value.data.length,
            year: value.data.year,
            bitrate: value.data.bitrate,
            sample_rate: value.data.sample_rate,
            play_count: value.data.play_count,
            dbid: value.data.dbid,
            bpm: value.data.bpm,
            skip_count: value.data.skip_count,
            has_artwork: value.data.has_artwork,
            media_type: value.data.media_type,
            title: find_str_arg(&value, 1).map_or(String::new(), |a| a.val.clone()),
            location: find_str_arg(&value, 2).map_or(String::new(), |a| a.val.clone()),
            album: find_str_arg(&value, 3).map_or(String::new(), |a| a.val.clone()),
            artist: find_str_arg(&value, 4).map_or(String::new(), |a| a.val.clone()),
            genre: find_str_arg(&value, 5).map_or(String::new(), |a| a.val.clone()),
        }
    }
}

pub fn init_db() -> Database {
    Database::create(get_db()).unwrap()
}

pub fn insert_playlist(db: &Database, playlist: Playlist) -> Result<(), Error> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(PLAYLISTS)?;
        let uid = playlist.persistent_playlist_id;
        let data = bincode::serialize(&playlist).unwrap();
        table.insert(uid, data)?;
    }
    write_txn.commit()?;
    Ok(())
}

pub fn get_playlist(db: &Database, id: u64) -> Result<DBPlaylist, Error> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PLAYLISTS)?;
    let b = table.get(id)?.unwrap().value();
    let value: Playlist = bincode::deserialize(&b).unwrap();
    let playlist = DBPlaylist {
        persistent_playlist_id: value.persistent_playlist_id,
        timestamp: value.timestamp,
        title: value.title,
        is_master: value.is_master,
        tracks: value.tracks.iter().map(|id| get_track(db, *id)).filter(|t| t.is_ok()).map(|t| t.unwrap()).collect(),
    };
    Ok(playlist.into())
}

pub fn get_all_playlists(db: &Database) -> Result<Vec<DBPlaylist>, Error> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(PLAYLISTS)?;
    Ok(table
        .iter()
        .unwrap()
        .flatten()
        .map(|d| bincode::deserialize(&d.1.value()).unwrap())
        .collect::<Vec<Playlist>>()
        .iter()
        .map(|p| DBPlaylist{
            persistent_playlist_id: p.persistent_playlist_id,
            timestamp: p.timestamp,
            title: p.title.clone(),
            is_master: p.is_master,
            tracks: p.tracks.iter().map(|id| get_track(db, *id)).filter(|t| t.is_ok()).map(|t| t.unwrap()).collect()
        })
        .collect())
}

pub fn insert_track(db: &Database, track: Track) -> Result<(), Error> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(TRACKS)?;
        let uid = track.unique_id;
        let data = bincode::serialize(&track).unwrap();
        table.insert(uid, data)?;
        
        let read_txn = db.begin_read()?;
        let table = read_txn.open_table(PLAYLISTS)?;
        
        let pls = table
            .iter()
            .unwrap()
            .flatten()
            .map(|d| bincode::deserialize(&d.1.value()).unwrap())
            .collect::<Vec<Playlist>>();
        
        for pl in pls {
            if !pl.is_master { continue }
            let mut master = pl.clone();
            master.tracks.push(uid);
            insert_playlist(db, master);
            break;
        }
    }
    write_txn.commit()?;
    Ok(())
}

pub fn get_track(db: &Database, id: u32) -> Result<Track, Error> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(TRACKS)?;
    let b = table.get(id)?.unwrap().value();
    let track: Track = bincode::deserialize(&b).unwrap();
    Ok(track)
}

pub fn get_all_tracks(db: &Database) -> Result<Vec<Track>, Error> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(TRACKS)?;
    Ok(table
        .iter()
        .unwrap()
        .flatten()
        .map(|d| bincode::deserialize(&d.1.value()).unwrap())
        .collect::<Vec<Track>>())
}

pub fn get_last_track_id(db: &Database) -> Result<u32, Error> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(TRACKS)?;
    let l = table.last()?.map_or(80u32, |v| v.0.value());
    Ok(l)
}

// note: this hash function is used to make unique ids for each track. It doesn't aim to generate secure ones.
fn hash(buf: &[u8]) -> u64 {
    let mut hasher = Md5::new();
    hasher.update(buf);
    let arr = hasher.finalize()[..8].try_into().unwrap();
    u64::from_be_bytes(arr)
}
