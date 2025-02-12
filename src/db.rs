use std::fs::File;

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
    bitrate: u32,
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
    artist: String,
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
