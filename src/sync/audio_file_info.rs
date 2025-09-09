use itunesdb::xobjects::XTrackItem;
use serde::Deserialize;
use std::process::Stdio;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Deserialize, PartialEq)]
pub struct AudioInfo {
    streams: Vec<AudioStream>,
    format: AudioFormat,
}

pub struct FormattedAudio {
    pub sample_rate: u64,
    pub duration: f64,
    pub bit_rate: u64,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct AudioStream {
    codec_name: String,
    sample_rate: Option<String>,
    channels: Option<u8>,
    sample_fmt: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct AudioFormat {
    duration: String,
    size: String,
    bit_rate: String,
}

impl AudioInfo {
    pub fn get_nice_object(&self) -> FormattedAudio {
        FormattedAudio {
            bit_rate: self.format.bit_rate.parse().unwrap(),
            duration: self.format.duration.parse().unwrap(),
            sample_rate: self
                .get_non_image_stream()
                .sample_rate
                .as_ref()
                .unwrap()
                .parse()
                .unwrap(),
        }
    }

    pub fn get_audio_extension(&self) -> &str {
        match self.get_non_image_stream().codec_name.as_str() {
            "mp3" => "mp3",
            "alac" | "aac" => "m4a",
            _ => "wav",
        }
    }

    fn get_non_image_stream(&self) -> &AudioStream {
        self.streams
            .iter()
            .find(|i| i.codec_name != "mjpeg")
            .unwrap()
    }

    fn get_audio_codec(&self) -> String {
        match self.get_non_image_stream().codec_name.as_str() {
            "mp3" => "MPEG audio file",
            "aac" => "AAC audio file",
            "alac" => "Apple Lossless audio file",
            _ => "WAV audio file",
        }
        .to_string()
    }

    pub fn modify_xtrack(&self, track: &mut XTrackItem) {
        track.data.type1 = 0;
        track.data.type2 = if self.get_non_image_stream().codec_name == "mp3" {
            1
        } else {
            0
        };

        let bytes = match self.get_non_image_stream().codec_name.as_str() {
            "mp3" => "MP3",
            "aac" => "M4A",
            "alac" => "M4A ",
            _ => "WAV",
        }
        .as_bytes();

        let file_type = u32::from_be_bytes(if bytes.len() == 4 {
            [bytes[0], bytes[1], bytes[2], bytes[3]]
        } else {
            [bytes[0], bytes[1], bytes[2], 0u8]
        });

        track.data.filetype = file_type;

        track.update_arg(6, self.get_audio_codec());
    }
}

pub async fn from_path(p: &str) -> Option<AudioInfo> {
    let mut command = Command::new("ffprobe");
    command.arg("-i");
    command.arg(p);
    command.arg("-print_format");
    command.arg("json");
    command.arg("-v");
    command.arg("quiet");
    command.arg("-show_entries");
    command.arg("format=duration,size,bit_rate:stream=codec_name,width,height,sample_rate,channels,sample_fmt");
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());

    let mut child = command.spawn().unwrap();

    let mut vec = Vec::new();
    let stdout = child.stdout.take().unwrap();
    let size = BufReader::new(stdout)
        .read_to_end(&mut vec)
        .await
        .unwrap_or(0);
    if size == 0 {
        return None;
    }

    Some(serde_json::from_str(String::from_utf8_lossy(vec.as_slice()).as_ref()).unwrap())
}
