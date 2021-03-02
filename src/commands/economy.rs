use std::ops::Sub;

use crate::{
    constants::{DAILY_AMOUNT, GAMBLE_MULTIPLIERS, GAMBLE_WEIGHTS},
    data::PoolContainer,
    database::Guild,
    utils::basic_functions::format_seconds,
};
use chrono::prelude::*;
use rand::distributions::WeightedIndex;

use comfy_table::{Cell, CellAlignment::Center, ContentArrangement::Dynamic, Table};
use rand::prelude::*;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};

/// Check your current cowoins balance.
#[command]
async fn balance(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let member_id = msg.author.id;
    let data = ctx.data.read().await;
    let db = data.get::<PoolContainer>().unwrap();
    let content = match Guild::new(db, guild_id).get_member(member_id).await? {
        Some(member) => format!("You have {} cowoins", member.coins),
        None => format!("Could not find user with id: {}", member_id),
    };
    msg.reply(ctx, content).await?;

    Ok(())
}

/// Gamble your cowoins.
///
/// Usage: `gamble 100`
#[command]
#[num_args(1)]
async fn gamble(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let member_id = msg.author.id;
    let mut coins = args.single::<i64>()?;
    if coins < 1 {
        msg.reply(ctx, "Can't gamble with given amount").await?;

        return Ok(());
    }

    let data = ctx.data.read().await;
    let db = data.get::<PoolContainer>().unwrap();
    let guild = Guild::new(db, guild_id);
    if let Some(member) = guild.get_member(member_id).await? {
        if coins > member.coins {
            msg.reply(ctx, "You don't have enough balance").await?;
            return Ok(());
        }

        let multipliers = *GAMBLE_MULTIPLIERS;
        let weights = *GAMBLE_WEIGHTS;
        let dist = WeightedIndex::new(&weights).unwrap();
        let multiplier = multipliers[dist.sample(&mut thread_rng())];

        let new_balance = match multiplier == 0 {
            true => member.coins - coins,
            false => {
                coins = coins * multiplier;
                member.coins + coins
            }
        };

        let response = match multiplier {
            0 => format!("You lost {} cowoins, try again next time", coins),
            1 => format!("1x you gained {} cowoins", coins),
            2 => format!("2x you gained {} cowoins", coins),
            3 => format!("3x you gained {} cowoins", coins),
            4 => format!("4x you gained {} cowoins", coins),
            5 => format!("5x GODLIKE!!!! you gained {} cowoins", coins),
            _ => {
                msg.reply(ctx, "Something unexpected happned, try again later")
                    .await?;
                    
                return Ok(());
            }
        };
        guild
            .set_member_economy(member_id, new_balance, None)
            .await?;
        msg.reply(
            ctx,
            format!("{}\nYou have {} cowoins now", response, new_balance),
        )
        .await?;
    } else {
        msg.reply(ctx, format!("Could not find user with id: {}", member_id))
            .await?;
    }

    Ok(())
}

/// Grab your daily cowoins.
#[command]
async fn daily(ctx: &Context, msg: &Message) -> CommandResult {
    let daily_const = *DAILY_AMOUNT;
    let guild_id = msg.guild_id.unwrap();
    let member_id = msg.author.id;
    let data = ctx.data.read().await;
    let db = data.get::<PoolContainer>().unwrap();
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
                    },
                    false => format!(
                        "Wait another {} to redeem your daily cowoins",
                        format_seconds(86400 - difference as u64)
                    )
                }
            }
            None => {
                format!("Could not find user with id: {}", member_id)
            }
        }
    };
    msg.reply(ctx, content).await?;

    Ok(())
}

/// Cowoins leaderboard.
#[command]
async fn leaderboard(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    let db = data.get::<PoolContainer>().unwrap();
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
            .http
            .get_member(*guild_id.as_u64(), member.id as u64)
            .await?;
        table.add_row(vec![
            Cell::new(i + 1).set_alignment(Center),
            Cell::new(_member.display_name()).set_alignment(Center),
            Cell::new(member.coins).set_alignment(Center),
        ]);
    }
    msg.channel_id
        .send_message(ctx, |m| {
            m.reference_message(msg);
            m.embed(|e| {
                e.title("Leaderboard");
                e.description(format!("```\n{}\n```", table))
            })
        })
        .await?;

    Ok(())
}
