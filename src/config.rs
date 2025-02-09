use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct YouTubeConfiguration {
    pub user_id: u64
}

#[derive(Debug, Deserialize)]
struct SoundCloudConfiguration {
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