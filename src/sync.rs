use std::path::{Path, PathBuf};

use itunesdb::xobjects::XDatabase;
use soundcloud::sobjects::CloudPlaylists;
use tokio::{fs::File, io::{AsyncReadExt, AsyncWriteExt}, sync::mpsc::{Sender, UnboundedReceiver}};
use tokio_util::sync::CancellationToken;

use crate::{config::{get_config_path, get_configs_dir, get_temp_dl_dir, get_temp_itunesdb, LyricaConfiguration}, dlp::{self, DownloadProgress}};

pub enum AppEvent {
    SearchIPod,
    IPodFound(String),
    IPodNotFound,
    ParseItunes(String),
    ITunesParsed(XDatabase),
    SoundcloudGot(CloudPlaylists),
    DownloadPlaylist(String),
    CurrentProgress(DownloadProgress),
    OverallProgress((u32, u32))
}

pub fn initialize_async_service(sender: Sender<AppEvent>, receiver: UnboundedReceiver<AppEvent>, token: CancellationToken) {
    tokio::spawn(async move {
        let mut receiver = receiver;
        loop {
            tokio::select! {
                _ = token.cancelled() => { return; }
                r = receiver.recv() => {
                    if let Some(request) = r {
                        match request {
                            AppEvent::SearchIPod => {
                                /*if let Some(p) = util::search_ipod() {
                                    let _ = sender.send(AppEvent::IPodFound(p)).await;
                                } else {
                                    let _ = sender.send(AppEvent::IPodNotFound).await;
                                }*/
                                let _ = sender.send(AppEvent::IPodFound("/Users/michael/Documents/ipod/iTunes/iTunesDB".to_string())).await;
                            },
                            AppEvent::ParseItunes(path) => {
                                // todo: parse itunes
                                let _ = std::fs::create_dir_all(get_configs_dir());
                                let cd = get_temp_itunesdb();
                                let mut p: PathBuf = Path::new(&path).into();
                               // p.push("iPod_Control");
                             //   p.push("iTunes");
                              //  p.set_file_name("iTunesDB");
                                let _ = std::fs::copy(p, &cd);
                                let mut file = File::open(cd).await.unwrap();
                                let mut contents = vec![];
                                file.read_to_end(&mut contents).await.unwrap();
                                let xdb = itunesdb::deserializer::parse_bytes(&contents);
                                let _ = sender.send(AppEvent::ITunesParsed(xdb)).await;

                                let p = get_config_path();
                                if !p.exists() { 
                                    let config = LyricaConfiguration::default();
                                    let cfg_str = toml::to_string_pretty(&config).unwrap();
                                    let mut file = File::create(&p).await.unwrap();
                                    file.write(cfg_str.as_bytes()).await;
                                }
                                let mut file = File::open(p).await.unwrap();
                                let mut content = String::new();
                                file.read_to_string(&mut content).await.unwrap();
                                let config: LyricaConfiguration = toml::from_str(&content).unwrap();
                                
                                let app_version = soundcloud::get_app().await.unwrap().unwrap();
                                let client_id = soundcloud::get_client_id().await.unwrap().unwrap();
                                let playlists = soundcloud::get_playlists(config.get_soundcloud().user_id, client_id, app_version).await.unwrap();

                                let _ = sender.send(AppEvent::SoundcloudGot(playlists)).await;
                            },
                            AppEvent::DownloadPlaylist(playlist_url) => {
                                let _ = dlp::download_from_soundcloud(&playlist_url, &get_temp_dl_dir(), sender.clone()).await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });
}