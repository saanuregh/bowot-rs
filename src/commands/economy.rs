use std::ops::Sub;

use chrono::prelude::*;
use comfy_table::{Cell, CellAlignment::Center, ContentArrangement::Dynamic, Table};
use poise;
use rand::{distributions::WeightedIndex, prelude::*};

use crate::{
	constants::{DAILY_AMOUNT, GAMBLE_MULTIPLIERS, GAMBLE_WEIGHTS},
	data::PgPoolContainer,
	database::Guild,
	types::{Error, PoiseContext},
	utils::{
		discord::{guild_check, reply_embed, reply_plain},
		helpers::format_seconds,
	},
};

/// Check your current cowoins balance.
#[poise::command(slash_command)]
pub async fn balance(ctx: PoiseContext<'_>) -> Result<(), Error> {
	let guild = guild_check(ctx).await?;
	let guild_id = guild.id;
	let member_id = ctx.author().id;
	let data = ctx.discord().data.read().await;
	let db = data.get::<PgPoolContainer>().unwrap();
	let content = match Guild::new(db, guild_id).get_member(member_id).await? {
		Some(member) => format!("You have {} cowoins", member.coins),
		None => format!("Could not find user with id: {}", member_id),
	};
	reply_plain(ctx, content).await?;

	Ok(())
}

/// Gamble your cowoins.
///
/// Usage: `gamble 100`
#[poise::command(slash_command)]
pub async fn gamble(
	ctx: PoiseContext<'_>,
	#[description = "Amount to gamble"] coins: i64,
) -> Result<(), Error> {
	let guild = guild_check(ctx).await?;
	let guild_id = guild.id;
	let member_id = ctx.author().id;
	if coins < 1 {
		reply_plain(ctx, "Can't gamble with given amount").await?;

		return Ok(());
	}

	let data = ctx.discord().data.read().await;
	let db = data.get::<PgPoolContainer>().unwrap();
	let guild = Guild::new(db, guild_id);
	if let Some(member) = guild.get_member(member_id).await? {
		if coins > member.coins {
			reply_plain(ctx, "You don't have enough balance").await?;
			return Ok(());
		}

		let multipliers = GAMBLE_MULTIPLIERS;
		let weights = GAMBLE_WEIGHTS;
		let dist = WeightedIndex::new(&weights).unwrap();
		let multiplier = multipliers[dist.sample(&mut thread_rng())];

		let change = coins * multiplier;
		let new_balance = match multiplier == 0 {
			true => member.coins - coins,
			false => member.coins + change,
		};

		let response = match multiplier {
			0 => format!("You lost {} cowoins, try again next time", coins),
			1 => format!("1x you gained {} cowoins", change),
			2 => format!("2x you gained {} cowoins", change),
			3 => format!("3x you gained {} cowoins", change),
			4 => format!("4x you gained {} cowoins", change),
			5 => format!("5x GODLIKE!!!! you gained {} cowoins", change),
			_ => {
				reply_plain(ctx, "Something unexpected happned, try again later").await?;

				return Ok(());
			}
		};
		guild
			.set_member_economy(member_id, new_balance, None)
			.await?;
		reply_plain(
			ctx,
			format!("{}\nYou have {} cowoins now", response, new_balance),
		)
		.await?;
	} else {
		reply_plain(ctx, format!("Could not find user with id: {}", member_id)).await?;
	}

	Ok(())
}

/// Grab your daily cowoins.
#[poise::command(slash_command)]
pub async fn daily(ctx: PoiseContext<'_>) -> Result<(), Error> {
	let daily_const = DAILY_AMOUNT;
	let guild = guild_check(ctx).await?;
	let guild_id = guild.id;
	let member_id = ctx.author().id;
	let data = ctx.discord().data.read().await;
	let db = data.get::<PgPoolContainer>().unwrap();
	let guild = Guild::new(db, guild_id);
	let content = {
		match guild.get_member(member_id).await? {
			Some(member) => {
				let difference = Utc::now().sub(member.last_daily).num_seconds();
				match difference > 86400 {
					true => {
						let new_balance = member.coins + daily_const;
						guild
							.set_member_economy(member_id, new_balance, Some(Utc::now()))
							.await?;
						format!(
							"You have redeemed your daily {} cowoins, your balance is {}",
							daily_const, new_balance
						)
					}
					false => format!(
						"Wait another {} to redeem your daily cowoins",
						format_seconds(86400 - difference as u64)
					),
				}
			}
			None => {
				format!("Could not find user with id: {}", member_id)
			}
		}
	};
	reply_plain(ctx, content).await?;

	Ok(())
}

/// Cowoins leaderboard.
#[poise::command(slash_command)]
pub async fn leaderboard(ctx: PoiseContext<'_>) -> Result<(), Error> {
	let guild = guild_check(ctx).await?;
	let guild_id = guild.id;
	let data = ctx.discord().data.read().await;
	let db = data.get::<PgPoolContainer>().unwrap();
	let mut members = Guild::new(db, guild_id).get_members().await?;
	let mut table = Table::new();
	table.force_no_tty().enforce_styling();
	table.set_content_arrangement(Dynamic).set_table_width(100);
	table.set_header(vec![
		Cell::new("#").set_alignment(Center),
		Cell::new("User").set_alignment(Center),
		Cell::new("Cowoins").set_alignment(Center),
	]);
	members.sort_by(|a, b| b.coins.cmp(&a.coins));
	for (i, member) in members.iter().enumerate() {
		let _member = ctx
			.discord()
			.http
			.get_member(*guild_id.as_u64(), member.id as u64)
			.await?;
		table.add_row(vec![
			Cell::new(i + 1).set_alignment(Center),
			Cell::new(_member.display_name()).set_alignment(Center),
			Cell::new(member.coins).set_alignment(Center),
		]);
	}
	reply_embed(ctx, |e| {
		e.title("Leaderboard");
		e.description(format!("```\n{}\n```", table))
	})
	.await?;

	Ok(())
}
