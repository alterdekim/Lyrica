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

pub fn get_db() -> PathBuf {
    let mut p = get_configs_dir();
    p.push("data.redb");
    p
}

#[derive(Debug, Deserialize, Serialize)]
pub struct YouTubeConfiguration {
    pub user_id: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SoundCloudConfiguration {
    pub user_id: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LyricaConfiguration {
    soundcloud: SoundCloudConfiguration,
    youtube: YouTubeConfiguration,
}

impl Default for LyricaConfiguration {
    fn default() -> Self {
        Self {
            soundcloud: SoundCloudConfiguration { user_id: 0 },
            youtube: YouTubeConfiguration { user_id: 0 },
        }
    }
}

impl LyricaConfiguration {
    pub fn get_soundcloud(&self) -> &SoundCloudConfiguration {
        &self.soundcloud
    }

    pub fn get_youtube(&self) -> &YouTubeConfiguration {
        &self.youtube
    }
}
