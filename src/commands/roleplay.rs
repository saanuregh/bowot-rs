use poise::serenity_prelude::{Mentionable, User};

use crate::{
	types::{Error, PoiseContext},
	utils::{
		apis::neko_api,
		discord::{reply_embed, reply_plain},
	},
};

async fn _neko_command(ctx: PoiseContext<'_>, user: User, key: &str) -> Result<(), Error> {
	let title_builder = match key {
		"baka" => "{} calls {} baka",
		"cuddle" => "{} cuddles {}",
		"hug" => "{} hugs {}",
		"kiss" => "{} kisses {}",
		"pat" => "{} pats {}",
		"poke" => "{} pokes {}",
		"slap" => "{} slaps {}",
		"smug" => "{} smugs at {}",
		"tickle" => "{} tickles {}",
		_ => {
			reply_plain(ctx, "Wrong key").await?;
			return Ok(());
		}
	};
	let user_1 = ctx.author().mention();
	let user_2 = user.mention();
	let resp = neko_api(key, true).await?;
	if let Some(url) = resp.get("url") {
		reply_embed(ctx, |e| {
			e.description(
				title_builder
					.replacen("{}", &user_1.to_string(), 1)
					.replacen("{}", &user_2.to_string(), 1),
			);
			e.image(url)
		})
		.await?;
		return Ok(());
	} else {
		reply_plain(ctx, "Can't find a gif").await?;
	}

	Ok(())
}

/// Call somebody baka.
///
/// Usage: `baka @user`
#[poise::command(slash_command, defer_response)]
pub async fn baka(ctx: PoiseContext<'_>, #[description = "Who?"] user: User) -> Result<(), Error> {
	return _neko_command(ctx, user, "baka").await;
}

/// Cuddle somebody.
///
/// Usage: `cuddle @user`
#[poise::command(slash_command, defer_response)]
pub async fn cuddle(ctx: PoiseContext<'_>, #[description = "Who?"] user: User) -> Result<(), Error> {
	return _neko_command(ctx, user, "cuddle").await;
}

/// Hug somebody.
///
/// Usage: `hug @user`
#[poise::command(slash_command, defer_response)]
pub async fn hug(ctx: PoiseContext<'_>, #[description = "Who?"] user: User) -> Result<(), Error> {
	return _neko_command(ctx, user, "hug").await;
}

/// Kiss somebody.
///
/// Usage: `kiss @user`
#[poise::command(slash_command, defer_response)]
pub async fn kiss(ctx: PoiseContext<'_>, #[description = "Who?"] user: User) -> Result<(), Error> {
	return _neko_command(ctx, user, "kiss").await;
}

/// Pat somebody.
///
/// Usage: `pat @user`
#[poise::command(slash_command, defer_response)]
pub async fn pat(ctx: PoiseContext<'_>, #[description = "Who?"] user: User) -> Result<(), Error> {
	return _neko_command(ctx, user, "pat").await;
}

/// Poke somebody.
///
/// Usage: `poke @user`
#[poise::command(slash_command, defer_response)]
pub async fn poke(ctx: PoiseContext<'_>, #[description = "Who?"] user: User) -> Result<(), Error> {
	return _neko_command(ctx, user, "poke").await;
}

/// Slap somebody.
///
/// Usage: `slap @user`
#[poise::command(slash_command, defer_response)]
pub async fn slap(ctx: PoiseContext<'_>, #[description = "Who?"] user: User) -> Result<(), Error> {
	return _neko_command(ctx, user, "slap").await;
}

/// Smug at somebody.
///
/// Usage: `smug @user`
#[poise::command(slash_command, defer_response)]
pub async fn smug(ctx: PoiseContext<'_>, #[description = "Who?"] user: User) -> Result<(), Error> {
	return _neko_command(ctx, user, "smug").await;
}

/// Tickle somebody.
///
/// Usage: `tickle @user`
#[poise::command(slash_command, defer_response)]
pub async fn tickle(ctx: PoiseContext<'_>, #[description = "Who?"] user: User) -> Result<(), Error> {
	return _neko_command(ctx, user, "tickle").await;
}
