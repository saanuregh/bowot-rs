use crate::{data::SoundStore, voice::join_voice_channel};
use rand::seq::SliceRandom;
use rand::Rng;
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::channel::Message,
    prelude::Context,
};
use songbird::SongbirdKey;

/// Sheesh.
///
/// Usage: `sheesh`
#[command]
async fn sheesh(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager = data
        .get::<SongbirdKey>()
        .expect("Expected Songbird in TypeMap");
    let handler_lock = match manager.get(guild_id) {
        Some(hl) => hl,
        None => match join_voice_channel(ctx, msg).await {
            Some(hl) => hl,
            None => {
                msg.reply(
                    ctx,
                    format!(
                        "S{}sh!",
                        str::repeat("e", rand::thread_rng().gen_range(5..20))
                    ),
                )
                .await?;
                return Ok(());
            }
        },
    };
    let mut handler = handler_lock.lock().await;
    let sources_lock = data
        .get::<SoundStore>()
        .cloned()
        .expect("Sound cache was installed at startup.");
    let sources = sources_lock.lock().await;
    let sheesh_keys: Vec<&String> = sources.keys().filter(|x| x.contains("sheesh")).collect();
    let mut success = false;
    if let Some(chosen_sheesh) = sheesh_keys.choose(&mut rand::thread_rng()) {
        if let Some(source) = sources.get(chosen_sheesh.clone()) {
            handler.play_source(source.new_handle().into());
            success = true
        }
    }
    if !success {
        msg.reply(ctx, "Sheesh missing!!! AAAAAAAAAAAAAAAAAAAAAAAAA!")
            .await?;
        return Ok(());
    }

    msg.react(ctx, '✅').await?;
    Ok(())
}
