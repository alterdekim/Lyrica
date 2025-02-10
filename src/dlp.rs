use std::path::PathBuf;

use tokio::process::Command;

pub async fn download_from_soundcloud(playlist_url: &str, download_dir: &PathBuf) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = &[
            "--ignore-errors", 
            "--newline", 
            "--progress-template", 
            "{\"progressPercentage\":\"%(progress._percent_str)s\",\"progressTotal\":\"%(progress._total_bytes_str)s\",\"speed\":\"%(progress._speed_str)s\",\"ETA\":\"%(progress._eta_str)s\"}", 
            "-o", 
            "%(id)i.%(ext)s", 
            "--write-thumbnail", 
            playlist_url
    ];

    let mut command = Command::new("yt-dlp");
    command.args(args);
    command.current_dir(download_dir);

    let mut child = command.spawn()?;

    let mut stdout = Vec::new();
    let child_stdout = child.stdout.take();
    tokio::io::copy(&mut child_stdout.unwrap(), &mut stdout).await.unwrap();

    Ok(())
}