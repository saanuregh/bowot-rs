use poise::{
	send_reply,
	serenity::builder::CreateEmbed,
	serenity_prelude::{ChannelId, Colour, Guild, SerenityError},
	ReplyHandle,
};
use serde::Serialize;
use serde_json::json;
use sysinfo::{get_current_pid, ProcessExt, RefreshKind, System, SystemExt};
use tokio::time::Instant;

use crate::{
	types::{PoiseContext, SerenityContext},
	utils::helpers::format_seconds,
	Uptime,
};

#[derive(Clone, Serialize)]
pub struct MetaInfoResult {
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

pub async fn get_rest_latency(
	ctx: &SerenityContext,
	channel_id: ChannelId,
) -> anyhow::Result<u128> {
	let map = json!({"content" : "Calculating latency..."});
	let now = Instant::now();
	let message = ctx.http.send_message(channel_id.0, &map).await?;
	let rest_latency = now.elapsed().as_millis();
	message.delete(&ctx.http).await?;
	Ok(rest_latency)
}

pub async fn get_uptime(ctx: &SerenityContext) -> String {
	let data = ctx.data.read().await;
	let instant = data.get::<Uptime>().unwrap();
	let duration = instant.elapsed();
	format_seconds(duration.as_secs())
}

pub fn get_process_usage() -> (f32, u64) {
	let pid = get_current_pid().unwrap();
	let s = System::new_with_specifics(RefreshKind::new().with_processes());
	let p = s.process(pid).unwrap();
	(p.cpu_usage(), p.memory())
}

pub async fn get_meta_info(ctx: &SerenityContext) -> MetaInfoResult {
	let uptime = get_uptime(ctx).await;
	let (hoster_tag, hoster_id) = {
		let app_info = ctx.http.get_current_application_info().await.unwrap();
		(app_info.owner.tag(), app_info.owner.id.as_u64().clone())
	};
	let (cpu_usage, memory_usage) = get_process_usage();
	let current_user = ctx.cache.current_user();
	let bot_name = current_user.name.clone();
	let bot_icon = current_user
		.avatar_url()
		.unwrap_or(current_user.default_avatar_url());
	let num_guilds = ctx.cache.guilds().len();
	let num_shards = ctx.cache.shard_count();
	let num_channels = ctx.cache.guild_channel_count();
	let num_priv_channels = ctx.cache.private_channels().len();
	let version = env!("CARGO_PKG_VERSION");
	MetaInfoResult {
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

pub async fn reply<S: ToString>(
	ctx: PoiseContext<'_>,
	msg: S,
) -> Result<ReplyHandle<'_>, SerenityError> {
	send_reply(ctx, |m| {
		m.embed(|e| e.colour(Colour(0xbf5c4e)).description(msg))
	})
	.await
}

pub async fn reply_plain<S: ToString>(
	ctx: PoiseContext<'_>,
	msg: S,
) -> Result<ReplyHandle<'_>, SerenityError> {
	send_reply(ctx, |m| m.content(msg.to_string())).await
}

pub async fn reply_embed(
	ctx: PoiseContext<'_>,
	embed: impl FnOnce(&mut CreateEmbed) -> &mut CreateEmbed,
) -> Result<ReplyHandle<'_>, SerenityError> {
	send_reply(ctx, |m| m.embed(|e| embed(e.colour(Colour(0xbf5c4e))))).await
}

pub async fn guild_check(ctx: PoiseContext<'_>) -> anyhow::Result<Guild> {
	match ctx.guild() {
		Some(guild) => Ok(guild),
		None => {
			reply(ctx, "You must use this command from within a server.").await?;
			Err(anyhow::anyhow!(
				"You must use this command from within a server."
			))
		}
	}
}
