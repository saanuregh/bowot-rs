use crate::{data::PgPoolContainer, database::HydrateReminder};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
    prelude::Context,
};

async fn _cr_hydrate(ctx: &Context, msg: &Message, create: bool) -> CommandResult {
    let data = ctx.data.read().await;
    let db = data.get::<PgPoolContainer>().unwrap();
    let hydrate_reminder = HydrateReminder::new(db);
    let content = if create {
        hydrate_reminder.insert(msg.author.id).await?;
        "You are offically part of hydration now"
    } else {
        hydrate_reminder.delete(msg.author.id).await?;
        "Hope to see you in hydration again, Bye!"
    };
    msg.reply(ctx, content)
        .await?;
    Ok(())
}

/// Add yourself to hydrate reminder.
#[command("add")]
async fn add_hydrate(ctx: &Context, msg: &Message) -> CommandResult {
    _cr_hydrate(ctx,msg,true).await
}

/// Remove yourself from hydrate reminder.
#[command("remove")]
async fn remove_hydrate(ctx: &Context, msg: &Message) -> CommandResult {
    _cr_hydrate(ctx,msg,false).await
}
