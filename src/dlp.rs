use std::{path::PathBuf, process::Stdio};

use regex::Regex;
use serde::Deserialize;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::Command,
    sync::mpsc::Sender,
};

use crate::sync::AppEvent;

#[derive(Debug, Deserialize)]
pub struct DownloadProgress {
    pub progress_percentage: String,
    pub progress_total: String,
    pub speed: String,
    pub eta: String,
}

pub async fn download_track_from_soundcloud(
    track_url: &str,
    download_dir: &PathBuf,
    sender: Sender<AppEvent>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let _ = sender
        .send(AppEvent::SwitchScreen(crate::AppState::LoadingScreen))
        .await;

    if download_dir.exists() {
        let _ = std::fs::remove_dir_all(download_dir);
    }
    let _ = std::fs::create_dir_all(download_dir);

    let args = &[
        "-f",
        "mp3",
        "--ignore-errors",
        "--newline",
        "--progress-template",
        "{\"progress_percentage\":\"%(progress._percent_str)s\",\"progress_total\":\"%(progress._total_bytes_str)s\",\"speed\":\"%(progress._speed_str)s\",\"eta\":\"%(progress._eta_str)s\"}",
        "-o",
        "%(id)i.%(ext)s",
        "--write-thumbnail",
        track_url
    ];

    let mut command = Command::new("yt-dlp");
    command.args(args);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    command.current_dir(download_dir);

    let mut child = command.spawn()?;

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();

    while let Some(line) = reader.next_line().await? {
        if line.starts_with("{") {
            let progress: DownloadProgress = serde_json::from_str(&line).unwrap();
            let _ = sender.send(AppEvent::OverallProgress((0, 1))).await;
            let _ = sender.send(AppEvent::CurrentProgress(progress)).await;
        }
    }
    let _ = sender.send(AppEvent::OverallProgress((1, 1))).await;
    Ok(())
}

pub async fn download_from_soundcloud(
    playlist_url: &str,
    download_dir: &PathBuf,
    sender: Sender<AppEvent>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let _ = sender
        .send(AppEvent::SwitchScreen(crate::AppState::LoadingScreen))
        .await;
    let dl_rx: Regex = Regex::new(r"\[download\] Downloading item \d+ of \d+").unwrap();

    if download_dir.exists() {
        let _ = std::fs::remove_dir_all(download_dir);
    }
    let _ = std::fs::create_dir_all(download_dir);

    let args = &[
            "-f",
            "mp3",
            "--ignore-errors", 
            "--newline", 
            "--progress-template", 
            "{\"progress_percentage\":\"%(progress._percent_str)s\",\"progress_total\":\"%(progress._total_bytes_str)s\",\"speed\":\"%(progress._speed_str)s\",\"eta\":\"%(progress._eta_str)s\"}", 
            "-o", 
            "%(id)i.%(ext)s", 
            "--write-thumbnail", 
            playlist_url
    ];

    let mut command = Command::new("yt-dlp");
    command.args(args);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    command.current_dir(download_dir);

    let mut child = command.spawn()?;

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout).lines();

    while let Some(line) = reader.next_line().await? {
        match dl_rx.find(&line) {
            Some(m) => {
                let mut s = m.as_str();
                s = s.split("Downloading item ").last().unwrap();
                let s: Vec<&str> = s.split(' ').collect();
                let cur = s.first().unwrap().trim().parse().unwrap();
                let max = s.last().unwrap().trim().parse().unwrap();
                let _ = sender.send(AppEvent::OverallProgress((cur, max))).await;
            }
            None => {
                if line.starts_with("{") {
                    let progress: DownloadProgress = serde_json::from_str(&line).unwrap();
                    let _ = sender.send(AppEvent::CurrentProgress(progress)).await;
                }
            }
        }
    }

    Ok(())
}
