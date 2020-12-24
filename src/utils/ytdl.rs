// simplified async fork of https://github.com/GyrosOfWar/youtube-dl-rs

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{error::Error as StdError, process::Stdio, time::Duration};
use tokio::process::Command;

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
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct SingleVideo {
    pub album_artist: Option<String>,
    pub alt_title: Option<String>,
    pub artist: Option<String>,
    pub channel: Option<String>,
    pub channel_id: Option<String>,
    pub channel_url: Option<String>,
    pub description: Option<String>,
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
    pub url: Option<String>,
    pub view_count: Option<i64>,
    pub webpage_url: Option<String>,
}

#[derive(Debug)]
pub struct ExitCode {
    pub code: i32,
    pub stderr: String,
}

/// Errors that can occur during executing `youtube-dl` or during parsing the output.
#[derive(Debug)]
pub enum YoutubeDlError {
    /// I/O error
    Io(std::io::Error),
    /// Error parsing JSON
    Json(serde_json::Error),
    ExitCode(ExitCode),
    /// Process-level timeout expired.
    ProcessTimeout,
}

impl From<std::io::Error> for YoutubeDlError {
    fn from(err: std::io::Error) -> Self {
        YoutubeDlError::Io(err)
    }
}

impl From<serde_json::Error> for YoutubeDlError {
    fn from(err: serde_json::Error) -> Self {
        YoutubeDlError::Json(err)
    }
}

impl From<ExitCode> for YoutubeDlError {
    fn from(err: ExitCode) -> Self {
        YoutubeDlError::ExitCode(err)
    }
}

impl std::fmt::Display for YoutubeDlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {}", err),
            Self::Json(err) => write!(f, "json error: {}", err),
            Self::ExitCode(err) => {
                write!(
                    f,
                    "non-zero exit code: {}, stderr: {}",
                    err.code, err.stderr
                )
            }
            Self::ProcessTimeout => write!(f, "process timed out"),
        }
    }
}

impl StdError for YoutubeDlError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Json(err) => Some(err),
            Self::ExitCode(_) => None,
            Self::ProcessTimeout => None,
        }
    }
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
) -> Result<YoutubeDlOutput, YoutubeDlError> {
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
        match tokio::time::timeout(timeout, child).await {
            Ok(result) => match result {
                Ok(status) => status,
                Err(err) => {
                    return Err(YoutubeDlError::Io(err));
                }
            },
            Err(_) => {
                return Err(YoutubeDlError::ProcessTimeout);
            }
        }
    } else {
        child.await?
    };
    if exit_code.success() {
        let value: Value = serde_json::from_reader(stdout.as_slice())?;

        let is_playlist = value["_type"] == json!("playlist");
        if is_playlist {
            let playlist: Playlist = serde_json::from_value(value)?;
            Ok(YoutubeDlOutput::Playlist(Box::new(playlist)))
        } else {
            let video: SingleVideo = serde_json::from_value(value)?;
            Ok(YoutubeDlOutput::SingleVideo(Box::new(video)))
        }
    } else {
        Err(YoutubeDlError::ExitCode(ExitCode {
            code: exit_code.code().unwrap_or(1),
            stderr: "yrf".to_string(),
        }))
    }
}
