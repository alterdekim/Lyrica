use redb::{Database, Error, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};

use crate::config::get_db;

const TRACKS: TableDefinition<u32, Vec<u8>> = TableDefinition::new("tracks");

#[derive(Serialize, Deserialize)]
struct Track {
    unique_id: u32,
    filetype: u32,
    stars: u8,
    last_modified_time: u32,
    size: u32,
    length: u32,
    year: u32,
    bitrate: u32,
    sample_rate: u32,
    play_count: u32,
    dbid: u64,
    bpm: u16,
    skip_count: u32,
    has_artwork: u8,
    media_type: u32,
    title: String,
    location: String,
    album: String,
    artist: String,
    genre: String,
}

fn init_db() -> Database {
    Database::create(get_db()).unwrap()
}

fn insert_track(db: &Database, track: Track) -> Result<(), Error> {
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

fn get_track(db: &Database, id: u32) -> Result<Track, Error> {
    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(TRACKS)?;
    let b = table.get(id)?.unwrap().value();
    let track: Track = bincode::deserialize(&b).unwrap();
    Ok(track)
}
