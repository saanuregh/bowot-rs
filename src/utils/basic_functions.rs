use crate::{ShardManagerContainer, Uptime};
use lazy_static::lazy_static;
use num_format::{Locale, ToFormattedString};
use serde::Serialize;
use serde_json::json;
use serenity::{client::bridge::gateway::ShardId, model::channel::Message, prelude::Context};
use std::{fs::read_to_string, process::id};
use tokio::{process::Command, time::Instant};
use toml::Value;
use walkdir::WalkDir;

// Capitalizes the first letter of a str.
pub fn capitalize_first(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn shorten(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        None => s.to_string(),
        Some((idx, _)) => s[..idx].to_string(),
    }
}

pub fn string_to_seconds(text: impl ToString) -> u64 {
    let s = text.to_string();
    let words = s.split(' ');
    let mut seconds = 0;

    for i in words {
        if i.ends_with("s") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0);
        }
        if i.ends_with("m") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 60;
        }
        if i.ends_with("h") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 3600;
        }
        if i.ends_with("D") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 86_400;
        }
        if i.ends_with("W") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 604_800;
        }
        if i.ends_with("M") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 2_628_288;
        }
        if i.ends_with("Y") {
            let num = &i[..i.len() - 1];
            seconds += num.parse::<u64>().unwrap_or(0) * 31_536_000;
        }
    }

    seconds
}

#[derive(Default)]
struct CodeStruct {
    c_blank: u32,
    c_comment: u32,
    c_code: u32,
    c_lines: u32,
    command_count: u32,
}

lazy_static! {
    static ref CODE: CodeStruct = {
        let mut c = CodeStruct::default();
        for entry in WalkDir::new("src") {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let count = loc::count(path.to_str().unwrap());
                let text = read_to_string(&path).unwrap();
                c.command_count += text.match_indices("#[command]").count() as u32;
                c.c_blank += count.blank;
                c.c_comment += count.comment;
                c.c_code += count.code;
                c.c_lines += count.lines;
            }
        }
        c
    };
    static ref VERSION: String = {
        let data = include_str!("../../Cargo.toml").parse::<Value>().unwrap();
        let version = data["package"]["version"].as_str().unwrap();
        version.to_string()
    };
}

#[derive(Serialize)]
pub struct MetaInfoResult {
    pub shard_latency: String,
    pub uptime: String,
    pub full_mem: String,
    pub reasonable_mem: String,
    pub version: String,
    pub c_blank: u32,
    pub c_comment: u32,
    pub c_code: u32,
    pub c_lines: u32,
    pub command_count: u32,
    pub hoster_tag: String,
    pub hoster_id: u64,
    pub bot_name: String,
    pub bot_icon: Option<String>,
    pub num_guilds: usize,
    pub num_shards: u64,
    pub num_channels: usize,
    pub num_priv_channels: usize,
}

pub async fn get_rest_latency(ctx: &Context, channel_id: u64) -> serenity::Result<(u128, Message)> {
    let map = json!({"content" : "Calculating latency..."});
    let now = Instant::now();
    let message = ctx.http.send_message(channel_id, &map).await?;
    let rest_latency = now.elapsed().as_millis();
    Ok((rest_latency, message))
}

pub async fn get_shard_latency(ctx: &Context) -> String {
    let data = ctx.data.read().await;
    let shard_manager = data.get::<ShardManagerContainer>().unwrap();
    let manager = shard_manager.lock().await;
    let runners = manager.runners.lock().await;
    let runner_raw = runners.get(&ShardId(ctx.shard_id));
    if let Some(runner) = runner_raw {
        if let Some(ms) = runner.latency {
            return format!("{}", ms.as_millis());
        }
    }
    "?".to_string()
}

pub async fn get_uptime(ctx: &Context) -> String {
    let data = ctx.data.read().await;
    let instant = data.get::<Uptime>().unwrap();
    let duration = instant.elapsed();
    humantime::format_duration(duration).to_string()
}

pub async fn get_memory_usage() -> (String, String) {
    let pid = id().to_string();
    let full_stdout = Command::new("sh")
        .arg("-c")
        .arg(format!(r"pmap {} | tail -n 1 | awk '/[0-9]K/{{print $2}}'", &pid).as_str())
        .output()
        .await
        .expect("failed to execute process");
    let reasonable_stdout = Command::new("sh")
        .arg("-c")
        .arg(
            format!(
                "pmap {} | head -n 2 | tail -n 1 | awk '/[0-9]K/{{print $2}}'",
                &pid
            )
            .as_str(),
        )
        .output()
        .await
        .expect("failed to execute process");
    let mut full_mem = String::from_utf8(full_stdout.stdout).unwrap();
    let mut reasonable_mem = String::from_utf8(reasonable_stdout.stdout).unwrap();
    full_mem.pop();
    full_mem.pop();
    full_mem = full_mem
        .parse::<u32>()
        .expect("NaN")
        .to_formatted_string(&Locale::en);
    reasonable_mem.pop();
    reasonable_mem.pop();
    reasonable_mem = reasonable_mem
        .parse::<u32>()
        .expect("NaN")
        .to_formatted_string(&Locale::en);
    (full_mem, reasonable_mem)
}

pub async fn meta_info(ctx: &Context) -> MetaInfoResult {
    let shard_latency = get_shard_latency(ctx).await;
    let uptime = get_uptime(ctx).await;
    let (hoster_tag, hoster_id) = {
        let app_info = ctx.http.get_current_application_info().await.unwrap();
        (app_info.owner.tag(), app_info.owner.id.as_u64().clone())
    };
    let (full_mem, reasonable_mem) = get_memory_usage().await;
    let current_user = ctx.cache.current_user().await;
    let bot_name = current_user.name.clone();
    let bot_icon = current_user.avatar_url();
    let num_guilds = ctx.cache.guilds().await.len();
    let num_shards = ctx.cache.shard_count().await;
    let num_channels = ctx.cache.guild_channel_count().await;
    let num_priv_channels = ctx.cache.private_channels().await.len();
    MetaInfoResult {
        shard_latency,
        bot_icon,
        bot_name,
        full_mem,
        reasonable_mem,
        uptime,
        version: VERSION.clone(),
        hoster_id,
        hoster_tag,
        num_channels,
        num_guilds,
        num_priv_channels,
        num_shards,
        c_blank: CODE.c_blank,
        c_code: CODE.c_code,
        c_comment: CODE.c_comment,
        c_lines: CODE.c_lines,
        command_count: CODE.command_count,
    }
}
