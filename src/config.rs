use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct YouTubeConfiguration {
    pub user_id: u64
}

#[derive(Debug, Deserialize)]
pub struct SoundCloudConfiguration {
    pub user_id: u64
}

#[derive(Debug, Deserialize)]
pub struct LyricaConfiguration {
    soundcloud: SoundCloudConfiguration,
    youtube: YouTubeConfiguration
}

impl LyricaConfiguration {
    pub fn get_soundcloud(&self) -> &SoundCloudConfiguration {
        &self.soundcloud
    }

    pub fn get_youtube(&self) -> &YouTubeConfiguration {
        &self.youtube
    }
}