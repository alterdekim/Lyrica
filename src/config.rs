use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub fn get_configs_dir() -> PathBuf {
    let mut p = dirs::home_dir().unwrap();
    p.push(".lyrica");
    p
}

pub fn get_temp_dl_dir() -> PathBuf {
    let mut p = get_configs_dir();
    p.push("tmp");
    p
}

pub fn clear_temp_dl_dir() {
    let path = get_temp_dl_dir();
    let _ = std::fs::remove_dir_all(path);
}

pub fn get_config_path() -> PathBuf {
    let mut p = get_configs_dir();
    p.push("config");
    p.set_extension("toml");
    p
}

pub fn get_temp_itunesdb() -> PathBuf {
    let mut p = get_configs_dir();
    p.push("idb");
    p
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct YouTubeConfiguration {
    pub user_id: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct SoundCloudConfiguration {
    pub user_id: u64,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct LyricaConfiguration {
    soundcloud: SoundCloudConfiguration,
    youtube: YouTubeConfiguration,
}

impl LyricaConfiguration {
    pub fn get_soundcloud(&self) -> &SoundCloudConfiguration {
        &self.soundcloud
    }

    pub fn get_youtube(&self) -> &YouTubeConfiguration {
        &self.youtube
    }
}
