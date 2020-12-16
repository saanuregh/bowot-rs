use crate::{database::Guild, Database};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
    prelude::Context,
};

/// Add yourself to hydrate reminder.
#[command("add")]
async fn add_hydrate(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    Guild::from_db(db, guild_id)
        .await?
        .add_hydrate(msg.author.id)?
        .save_guild(db)
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
    let db = data.get::<Database>().unwrap();
    Guild::from_db(db, guild_id)
        .await?
        .remove_hydrate(msg.author.id)?
        .save_guild(db)
        .await?;
    msg.reply(ctx, "Hope to see you in hydration again, Bye!")
        .await?;
    Ok(())
}
