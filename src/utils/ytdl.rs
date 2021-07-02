// simplified async fork of https://github.com/GyrosOfWar/youtube-dl-rs

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::{process::Command, time::Duration};

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct Playlist {
    pub entries: Option<Vec<SingleVideo>>,
    pub extractor: Option<String>,
    pub extractor_key: Option<String>,
    pub id: Option<String>,
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub uploader_id: Option<String>,
    pub uploader_url: Option<String>,
    pub webpage_url: Option<String>,
    pub webpage_url_basename: Option<String>,
    pub _type: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct SingleVideo {
    pub album_artist: Option<String>,
    pub alt_title: Option<String>,
    pub artist: Option<String>,
    pub channel: Option<String>,
    pub channel_id: Option<String>,
    pub channel_url: Option<String>,
    pub duration: Option<Value>,
    pub ext: Option<String>,
    pub extractor: Option<String>,
    pub extractor_key: Option<String>,
    pub id: String,
    pub is_live: Option<bool>,
    pub start_time: Option<String>,
    pub thumbnail: Option<String>,
    pub timestamp: Option<i64>,
    pub title: String,
    pub upload_date: Option<String>,
    pub uploader: Option<String>,
    pub uploader_id: Option<String>,
    pub uploader_url: Option<String>,
    pub view_count: Option<i64>,
    pub webpage_url: Option<String>,
    // pub description: Option<String>,
    // pub url: Option<String>,
}

/// Data returned by `YoutubeDl::run`. Output can either be a single video or a playlist of videos.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum YoutubeDlOutput {
    /// Playlist result
    Playlist(Box<Playlist>),
    /// Single video result
    SingleVideo(Box<SingleVideo>),
}

pub async fn ytdl_info(
    query: impl Into<String>,
    process_timeout: Option<Duration>,
) -> anyhow::Result<YoutubeDlOutput> {
    let args = [
        "--default-search",
        "ytsearch1",
        "-f",
        "webm[abr>0]/bestaudio/best",
        "-R",
        "infinite",
        "--ignore-config",
        "-J",
        &query.into(),
    ];
    let mut child = Command::new("youtube-dl")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .args(&args)
        .spawn()?;

    // Continually read from stdout so that it does not fill up with large output and hang forever.
    // We don't need to do this for stderr since only stdout has potentially giant JSON.
    let mut stdout = Vec::new();
    let child_stdout = child.stdout.take();
    tokio::io::copy(&mut child_stdout.unwrap(), &mut stdout).await?;

    let exit_code = if let Some(timeout) = process_timeout {
        tokio::time::timeout(timeout, child.wait()).await??
    } else {
        child.wait().await?
    };
    if exit_code.success() {
        let value: Value = serde_json::from_reader(stdout.as_slice())?;
        serde_value_to_ytdl(value)
    } else {
        Err(anyhow::anyhow!("Error fetching query"))
    }
}

pub fn serde_value_to_ytdl(value: Value) -> anyhow::Result<YoutubeDlOutput> {
    let is_playlist = value["_type"] == json!("playlist");
    if is_playlist {
        let playlist: Playlist = serde_json::from_value(value)?;
        Ok(YoutubeDlOutput::Playlist(Box::new(playlist)))
    } else {
        let video: SingleVideo = serde_json::from_value(value)?;
        Ok(YoutubeDlOutput::SingleVideo(Box::new(video)))
    }
}
