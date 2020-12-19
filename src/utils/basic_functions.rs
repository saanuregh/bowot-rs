use crate::{ShardManagerContainer, Uptime};
use serde::Serialize;
use serde_json::{json, Value};
use serenity::{client::bridge::gateway::ShardId, model::channel::Message, prelude::Context};
use sysinfo::{get_current_pid, ProcessExt, RefreshKind, System, SystemExt};
use tokio::time::Instant;

// Capitalizes the first letter of a str.
pub fn capitalize_first(input: &str) -> String {
    let mut c = input.chars();
    match c.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
    }
}

pub fn format_seconds(seconds: u64) -> String {
    let d = seconds / 86_400;
    let h = seconds / 3600 % 24;
    let m = seconds % 3600 / 60;
    let s = seconds % 3600 % 60;
    let mut output = format!("{}s", s);
    if m != 0 {
        output = format!("{}m {}", m, output);
    }
    if h != 0 {
        output = format!("{}h {}", h, output);
    }
    if d != 0 {
        output = format!("{}D {}", d, output);
    }
    output
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

#[derive(Clone, Serialize)]
pub struct MetaInfoResult {
    pub shard_latency: String,
    pub uptime: String,
    pub memory_usage: u64,
    pub cpu_usage: f32,
    pub version: &'static str,
    pub hoster_tag: String,
    pub hoster_id: u64,
    pub bot_name: String,
    pub bot_icon: String,
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
    format_seconds(duration.as_secs())
}

pub fn get_process_usage() -> (f32, u64) {
    let pid = get_current_pid().unwrap();
    let s = System::new_with_specifics(RefreshKind::new().with_processes());
    let p = s.get_process(pid).unwrap();
    (p.cpu_usage(), p.memory())
}

pub async fn get_meta_info(ctx: &Context) -> MetaInfoResult {
    let shard_latency = get_shard_latency(ctx).await;
    let uptime = get_uptime(ctx).await;
    let (hoster_tag, hoster_id) = {
        let app_info = ctx.http.get_current_application_info().await.unwrap();
        (app_info.owner.tag(), app_info.owner.id.as_u64().clone())
    };
    let (cpu_usage, memory_usage) = get_process_usage();
    let current_user = ctx.cache.current_user().await;
    let bot_name = current_user.name.clone();
    let bot_icon = current_user
        .avatar_url()
        .unwrap_or(current_user.default_avatar_url());
    let num_guilds = ctx.cache.guilds().await.len();
    let num_shards = ctx.cache.shard_count().await;
    let num_channels = ctx.cache.guild_channel_count().await;
    let num_priv_channels = ctx.cache.private_channels().await.len();
    let version = env!("CARGO_PKG_VERSION");
    MetaInfoResult {
        shard_latency,
        bot_icon,
        bot_name,
        cpu_usage,
        memory_usage,
        uptime,
        version,
        hoster_id,
        hoster_tag,
        num_channels,
        num_guilds,
        num_priv_channels,
        num_shards,
    }
}

pub fn merge_json(a: &mut Value, b: &Value) {
    match (a, b) {
        (&mut Value::Object(ref mut a), &Value::Object(ref b)) => {
            for (k, v) in b {
                merge_json(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}
