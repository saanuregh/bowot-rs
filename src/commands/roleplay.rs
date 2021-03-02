use crate::utils::apis::*;
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::{channel::Message, misc::Mentionable},
    prelude::Context,
};

async fn _neko_command(ctx: &Context, msg: &Message, key: &str) -> CommandResult {
    if msg.mentions.len() != 1 {
        msg.reply(ctx, "You must mention 1 person").await?;

        return Ok(());
    }
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
            msg.reply(ctx, "Wrong key").await?;
            return Ok(());
        }
    };
    let user_1 = msg.author.mention();
    let user_2 = msg.mentions[0].mention();
    let resp = neko_api(key, true).await?;
    if let Some(url) = resp.get("url") {
        msg.channel_id
            .send_message(ctx, |m| {
                m.embed(|e| {
                    e.description(
                        title_builder
                            .replacen("{}", &user_1.to_string(), 1)
                            .replacen("{}", &user_2.to_string(), 1),
                    );
                    e.image(url)
                })
            })
            .await?;
        return Ok(());
    }else {
        msg.reply(ctx, "Can't find a gif").await?;
    }

    Ok(())
}

/// Call somebody baka.
///
/// Usage: `baka @user`
#[command]
async fn baka(ctx: &Context, msg: &Message) -> CommandResult {
    return _neko_command(ctx, msg, "baka").await;
}

/// Cuddle somebody.
///
/// Usage: `cuddle @user`
#[command]
async fn cuddle(ctx: &Context, msg: &Message) -> CommandResult {
    return _neko_command(ctx, msg, "cuddle").await;
}

/// Hug somebody.
///
/// Usage: `hug @user`
#[command]
async fn hug(ctx: &Context, msg: &Message) -> CommandResult {
    return _neko_command(ctx, msg, "hug").await;
}

/// Kiss somebody.
///
/// Usage: `kiss @user`
#[command]
async fn kiss(ctx: &Context, msg: &Message) -> CommandResult {
    return _neko_command(ctx, msg, "kiss").await;
}

/// Pat somebody.
///
/// Usage: `pat @user`
#[command]
async fn pat(ctx: &Context, msg: &Message) -> CommandResult {
    return _neko_command(ctx, msg, "pat").await;
}

/// Poke somebody.
///
/// Usage: `poke @user`
#[command]
async fn poke(ctx: &Context, msg: &Message) -> CommandResult {
    return _neko_command(ctx, msg, "poke").await;
}

/// Slap somebody.
///
/// Usage: `slap @user`
#[command]
async fn slap(ctx: &Context, msg: &Message) -> CommandResult {
    return _neko_command(ctx, msg, "slap").await;
}

/// Smug at somebody.
///
/// Usage: `smug @user`
#[command]
async fn smug(ctx: &Context, msg: &Message) -> CommandResult {
    return _neko_command(ctx, msg, "smug").await;
}

/// Tickle somebody.
///
/// Usage: `tickle @user`
#[command]
async fn tickle(ctx: &Context, msg: &Message) -> CommandResult {
    return _neko_command(ctx, msg, "tickle").await;
}
