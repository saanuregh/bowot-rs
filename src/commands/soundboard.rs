use crate::{data::SoundStore, soundboard::get_all_keys, voice::join_voice_channel};
use rand::seq::SliceRandom;
use rand::Rng;
use serenity::framework::standard::Args;
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
    let sources = data
        .get::<SoundStore>()
        .expect("Sound cache was installed at startup.");
    let all_keys = get_all_keys(&sources);
    let sheesh_keys: Vec<&String> = all_keys.iter().filter(|k| k.contains("sheesh")).collect();
    let chosen_sheesh = match sheesh_keys.choose(&mut rand::thread_rng()) {
        Some(x) => sources.get(x.clone()),
        None => None,
    };
    if chosen_sheesh.is_none() {
        msg.reply(ctx, "Sheesh missing!!! AAAAAAAAAAAAAAAAAAAAAAAAA!")
            .await?;
        return Ok(());
    }
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
    handler_lock
        .lock()
        .await
        .play_source(chosen_sheesh.unwrap().new_handle().into());
    msg.react(ctx, '✅').await?;
    Ok(())
}

/// List or play a sound.
///
/// Usage: `soundboard imdying`
/// or `soundboard` to list all sound clips
#[command]
#[aliases(sb)]
async fn soundboard(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let query = args.message().to_string();
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let sources = data
        .get::<SoundStore>()
        .expect("Sound cache was installed at startup.");
    let mut all_keys = get_all_keys(&sources);
    all_keys.sort();
    if query.is_empty() {
        msg.reply(
            ctx,
            format!("__**Available sounds:**__\n`{}`", all_keys.join("`\n`")),
        )
        .await?;

        return Ok(());
    }

    if let Some(chosen_sound) = sources.get(&query) {
        let manager = data
            .get::<SongbirdKey>()
            .expect("Expected Songbird in TypeMap");
        let handler_lock = match manager.get(guild_id) {
            Some(hl) => hl,
            None => match join_voice_channel(ctx, msg).await {
                Some(hl) => hl,
                None => {
                    msg.reply(ctx, "Not in a voice channel").await?;
                    return Ok(());
                }
            },
        };
        handler_lock
            .lock()
            .await
            .play_source(chosen_sound.new_handle().into());
        msg.react(ctx, '✅').await?;

        return Ok(());
    }

    msg.reply(
        ctx,
        "Could not find the sound type `soundboard` to see all the available clips",
    )
    .await?;
    Ok(())
}
