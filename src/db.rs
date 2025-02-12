use std::fs::File;

use itunesdb::xobjects::{XArgument, XTrackItem};
use md5::{Digest, Md5};
use redb::{Database, Error, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use soundcloud::sobjects::CloudTrack;

use crate::config::{get_db, get_temp_dl_dir};

const TRACKS: TableDefinition<u32, Vec<u8>> = TableDefinition::new("tracks");

#[derive(Serialize, Deserialize)]
pub struct Track {
    pub unique_id: u32,
    filetype: u32,
    stars: u8,
    last_modified_time: u32,
    size: u32,
    length: u32,
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
    location: String,
    album: String,
    pub artist: String,
    genre: String,
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
            bitrate: header.bitrate.bps(),
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
            artist: "Soundcloud".to_string(),
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

// TODO: implement From (or Into) for Track, convert from Soundcloud Audio or iTunes

pub fn init_db() -> Database {
    Database::create(get_db()).unwrap()
}

pub fn insert_track(db: &Database, track: Track) -> Result<(), Error> {
    let write_txn = db.begin_write()?;
    {
        let mut table = write_txn.open_table(TRACKS)?;
        let uid = track.unique_id;
        let data = bincode::serialize(&track).unwrap();
        table.insert(uid, data)?;
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
