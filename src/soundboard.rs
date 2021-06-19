use dashmap::DashMap;
use songbird::{
    input::{cached::Compressed, ffmpeg},
    Bitrate,
};
use std::{fs::read_dir, path::PathBuf};
use tracing::info;

pub type AudioMap = DashMap<String, Compressed>;

const SOUNDS_DIR: &str = "sounds";

pub async fn get_compressed_source_from_path(path: PathBuf) -> Option<(String, Compressed)> {
    let name = path
        .clone()
        .file_stem()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    if let Ok(mut input) = ffmpeg(path).await {
        input.metadata.source_url = Some(format!("soundboard_{}", name));
        if let Ok(song_src) = Compressed::new(input, Bitrate::BitsPerSecond(128_000)) {
            let _ = song_src.raw.spawn_loader();
            return Some((name, song_src));
        }
    }
    None
}

pub async fn init_sound_store() -> AudioMap {
    let audio_map: AudioMap = DashMap::new();
    if let Ok(paths) = read_dir(SOUNDS_DIR) {
        for dir_entry_result in paths {
            if let Ok(dir_entry) = dir_entry_result {
                let path = dir_entry.path();
                if let Some(ext) = path.extension() {
                    if ext.eq("mp3") {
                        if let Some((name, song_src)) = get_compressed_source_from_path(path).await
                        {
                            audio_map.insert(name, song_src);
                        }
                    }
                }
            }
        }
    }
    info!("Cached {} audio files", audio_map.len());
    audio_map
}

pub fn get_all_keys(sources: &AudioMap) -> Vec<String> {
    sources.clone().into_iter().map(|(k, _)| k).collect()
}
