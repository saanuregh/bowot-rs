use crate::{database::Guild, utils::basic_functions::format_seconds, Database};
use chrono::prelude::*;
use comfy_table::{Cell, CellAlignment::Center, ContentArrangement::Dynamic, Table};
use rand::distributions::WeightedIndex;
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
    let db = data.get::<Database>().unwrap();
    let guild = Guild::from_db(db, guild_id).await?;
    let member = guild.get_member(member_id)?;
    msg.reply(ctx, format!("You have {} cowoins", member.coins))
        .await?;
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
    let db = data.get::<Database>().unwrap();
    let mut guild = Guild::from_db(db, guild_id).await?;
    let mut member = guild.get_member(member_id)?;

    if coins > member.coins {
        msg.reply(ctx, "You don't have enough balance").await?;
        return Ok(());
    }

    let multipliers = [0, 1, 2, 3, 4, 5];
    let weights = [6.0, 2.0, 1.7, 0.2, 0.1];
    let dist = WeightedIndex::new(&weights).unwrap();
    let multiplier = multipliers[dist.sample(&mut thread_rng())];
    if multiplier == 0 {
        member.update_coins(coins * -1);
    } else {
        coins = coins * multiplier;
        member.update_coins(coins);
    }

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
    guild.update_member(member.clone())?.save_guild(db).await?;
    msg.reply(
        ctx,
        format!("{}\nYou have {} cowoins now", response, member.coins),
    )
    .await?;
    Ok(())
}

/// Grab your daily cowoins.
#[command]
async fn daily(ctx: &Context, msg: &Message) -> CommandResult {
    let daily_const = 1000;
    let guild_id = msg.guild_id.unwrap();
    let member_id = msg.author.id;
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    let mut guild = Guild::from_db(db, guild_id).await?;
    let mut member = guild.get_member(member_id)?;
    let difference = Utc::now().timestamp() - member.last_daily;
    if difference > 86400 {
        member
            .update_coins(daily_const)
            .update_last_daily(Utc::now().timestamp());
        guild.update_member(member.clone())?.save_guild(db).await?;
        msg.reply(
            ctx,
            format!(
                "You have redeemed your daily {} cowoins, your balance is {}",
                daily_const, member.coins
            ),
        )
        .await?;
    } else {
        msg.reply(
            ctx,
            format!(
                "Wait another {} to redeem your daily cowoins",
                format_seconds(86400 - difference as u64)
            ),
        )
        .await?;
    }
    Ok(())
}

/// Cowoins leaderboard.
#[command]
async fn leaderboard(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    let mut members = Guild::from_db(db, guild_id).await?.members;
    let mut table = Table::new();
    table.force_no_tty().enforce_styling();
    table.set_content_arrangement(Dynamic).set_table_width(100);
    table.set_header(vec![
        Cell::new("#").set_alignment(Center),
        Cell::new("User").set_alignment(Center),
        Cell::new("Cowoins").set_alignment(Center),
    ]);
    members.sort_by(|a, b| b.coins.cmp(&a.coins));
    for (i, member) in members.clone().iter().enumerate() {
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
