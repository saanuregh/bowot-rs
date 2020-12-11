use crate::{database::Guild, MongoClient};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};

/// Add/Remove yourself to/from hydrate reminder.
///
/// Configurable aspects:
/// `add`: Add yourself to hydrate reminder.
/// `remove`: Remove yourself from hydrate reminder.
#[command]
#[only_in("guilds")]
#[sub_commands(add_hydrate, remove_hydrate)]
async fn hydrate(_ctx: &Context, _msg: &Message, _args: Args) -> CommandResult {
    Ok(())
}

/// Add yourself to hydrate reminder.
#[command("add")]
async fn add_hydrate(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    let client = data.get::<MongoClient>().unwrap();
    Guild::from_db(client, guild_id)
        .await?
        .add_hydrate(msg.author.id)?
        .save_guild(client)
        .await?;
    msg.reply(ctx, "You are offically part of hydration now")
        .await?;
    Ok(())
}

/// Remove yourself from hydrate reminder.
#[command("remove")]
async fn remove_hydrate(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    let client = data.get::<MongoClient>().unwrap();
    Guild::from_db(client, guild_id)
        .await?
        .remove_hydrate(msg.author.id)?
        .save_guild(client)
        .await?;
    msg.reply(ctx, "Hope to see you in hydration again, Bye!")
        .await?;
    Ok(())
}
