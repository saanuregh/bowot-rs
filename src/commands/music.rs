use crate::{
    data::RedisPoolContainer,
    utils::{
        basic_functions::shorten,
        ytdl::{ytdl_info, YoutubeDlOutput},
    },
    voice::{format_duration, get_now_playing_embed, join_voice_channel},
    ytdl_cache::YtdlCache,
};
use rand::Rng;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult, Delimiter},
    model::channel::Message,
};
use songbird::{
    input::{ytdl, Input},
    SongbirdKey,
};
use strum_macros::{EnumString, ToString};
use tracing::{error, info};

const JOIN_MSG: &str = "Please, connect the bot to the voice channel you are currently on first with the `join` command.";
const QUEUE_EMPTY_MSG: &str = "The queue is empty";
const NOTIN_VC_MSG: &str = "Not in a voice channel";
const NOTHING_PLAYING: &str = "Nothing playing";
const MAX_PLAYLIST: usize = 25;

/// Joins me to the voice channel you are currently on.
#[command]
#[aliases("connect")]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    if join_voice_channel(ctx, msg).await.is_some() {
        msg.react(ctx, '✅').await?;
    } else {
        msg.reply(ctx, NOTIN_VC_MSG).await?;
    }

    Ok(())
}

/// Disconnects me from the voice channel if im in one.
#[command]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager = data
        .get::<SongbirdKey>()
        .expect("Expected Songbird in TypeMap");
    if manager.get(guild_id).is_some() {
        if let Err(e) = manager.remove(guild_id).await {
            msg.reply(ctx, format!("Failed: {:?}", e)).await?;
        }
        msg.react(ctx, '✅').await?;
    } else {
        msg.reply(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

async fn _ytdl_push_into_source(sources: &mut Vec<Input>, result: YoutubeDlOutput) {
    match result {
        YoutubeDlOutput::Playlist(p) => {
            if let Some(playlist) = p.entries {
                for s in playlist.clone().into_iter().take(MAX_PLAYLIST) {
                    match ytdl(&s.webpage_url.clone().unwrap()).await {
                        Ok(mut source) => {
                            source.metadata.title = Some(s.title);
                            sources.push(source)
                        }
                        Err(why) => error!("Err starting source: {:?}", why),
                    }
                }
            }
        }
        YoutubeDlOutput::SingleVideo(s) => match ytdl(&s.webpage_url.clone().unwrap()).await {
            Ok(mut source) => {
                source.metadata.title = Some(s.title);
                sources.push(source)
            }
            Err(why) => error!("Err starting source: {:?}", why),
        },
    }
}

/// Adds a song to the queue.
///
/// Usage: `play starmachine2000`
/// or `play https://www.youtube.com/watch?v=dQw4w9WgXcQ`
#[command]
#[min_args(1)]
#[aliases(p)]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let query = args.message().to_string();
    if !msg.embeds.is_empty() {
        let _ = msg.clone().suppress_embeds(ctx).await;
    }
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
                msg.reply(ctx, NOTIN_VC_MSG).await?;

                return Ok(());
            }
        },
    };
    let loading_msg = msg.reply(ctx, "Loading...").await?;
    let redis_con = data
        .get::<RedisPoolContainer>()
        .expect("Expected RedisPoolContainer in TypeMap");
    let mut sources: Vec<Input> = Vec::new();
    let mut cache_hit = false;
    match YtdlCache::new(redis_con.clone(), query.clone(), None)
        .get()
        .await
    {
        Ok(result) => {
            cache_hit = true;
            info!("ytdl cache hit for {}", query);
            _ytdl_push_into_source(&mut sources, result).await;
        }
        Err(err) => error!("error fetching cache for query:{}\n{:?}", query, err),
    }

    if !cache_hit {
        match ytdl_info(query.clone(), None).await {
            Ok(result) => {
                info!("ytdl cache miss for {}", query);
                _ytdl_push_into_source(&mut sources, result.clone()).await;
                if let Err(err) = YtdlCache::new(redis_con.clone(), query.clone(), Some(result))
                    .set()
                    .await
                {
                    error!("error caching query:{}\n{:?}", query, err)
                };
            }
            Err(err) => error!("error fetching query:{}\n{:?}", query, err),
        }
    }

    let _ = loading_msg.delete(ctx).await;
    if sources.is_empty() {
        msg.reply(ctx, "Couldn't find any result for the query")
            .await?;

        return Ok(());
    }
    let mut handler = handler_lock.lock().await;
    let sources_len = sources.len();
    if sources_len > 1 {
        msg.reply(ctx, format!("__**Queued:**__  `{}` tracks", sources_len))
            .await?;
    } else {
        let metadata = sources.first().unwrap().metadata.as_ref().clone();
        if handler.queue().current().is_none() {
            msg.channel_id
                .send_message(ctx, |m| {
                    m.reference_message(msg);
                    get_now_playing_embed(m, metadata);
                    m
                })
                .await?;
        } else {
            msg.reply(
                ctx,
                format!(
                    "__**Queued:**__  `{}` | `{}`",
                    metadata.title.unwrap(),
                    format_duration(metadata.duration)
                ),
            )
            .await?;
        }
    }
    for source in sources {
        handler.enqueue_source(source)
    }

    Ok(())
}

/// Stops the current player (clears song queue).
#[command]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager = data
        .get::<SongbirdKey>()
        .expect("Expected Songbird in TypeMap");
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if queue.current().is_some() {
            queue.stop();
            msg.react(ctx, '✅').await?;
        } else {
            msg.reply(ctx, NOTHING_PLAYING).await?;
        }
    } else {
        msg.reply(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Show the song queue.
#[command]
async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager = data
        .get::<SongbirdKey>()
        .expect("Expected Songbird in TypeMap");
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue().current_queue();
        if !queue.is_empty() {
            let mut queue_str = String::new();
            let metadata = queue[0].metadata();
            queue_str += &format!(
                "__**Now playing:**__\n```yaml\n{} | {}\n```",
                shorten(&metadata.title.clone().unwrap(), 40),
                format_duration(metadata.duration)
            );
            if queue.len() > 1 {
                queue_str += "\n__**Queue:**__\n```yaml\n";
                for (index, track) in queue[1..].iter().take(10).enumerate() {
                    let metadata = track.metadata();
                    queue_str += &format!(
                        "{}: {} | {}\n",
                        index + 1,
                        shorten(&metadata.title.clone().unwrap(), 40),
                        format_duration(metadata.duration)
                    );
                }
                if queue.len() > 10 {
                    queue_str += &format!("... {}", queue.len());
                }
                queue_str += "\n```";
            }
            queue_str = queue_str.replace("@", "@\u{200B}");
            msg.channel_id
                .send_message(ctx.clone(), |m| {
                    m.reference_message(msg);
                    m.embed(|e| {
                        e.description(&queue_str);
                        e
                    })
                })
                .await?;
        } else {
            msg.reply(ctx, QUEUE_EMPTY_MSG).await?;
        }
    } else {
        msg.reply(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Clears the song queue.
#[command]
async fn clear_queue(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager = data
        .get::<SongbirdKey>()
        .expect("Expected Songbird in TypeMap");
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if !queue.is_empty() {
            queue.modify_queue(|q| q.truncate(1));
            msg.react(ctx, '✅').await?;
        } else {
            msg.reply(ctx, QUEUE_EMPTY_MSG).await?;
        }
    } else {
        msg.reply(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Shuffles the song queue.
#[command]
async fn shuffle(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager = data
        .get::<SongbirdKey>()
        .expect("Expected Songbird in TypeMap");
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if !queue.is_empty() {
            queue.modify_queue(|q| {
                let mut rng = rand::thread_rng();
                let mut i = q.len();
                while i >= 2 {
                    i -= 1;
                    q.swap(i, rng.gen_range(1..i + 1));
                }
            });
            msg.react(ctx, '✅').await?;
        } else {
            msg.reply(ctx, QUEUE_EMPTY_MSG).await?;
        }
    } else {
        msg.reply(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Skips the current song being played.
#[command]
#[aliases(next)]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager = data
        .get::<SongbirdKey>()
        .expect("Expected Songbird in TypeMap");
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if queue.current().is_some() {
            let _ = queue.skip();
            msg.react(ctx, '✅').await?;
        } else {
            msg.reply(ctx, NOTHING_PLAYING).await?;
        }
    } else {
        msg.reply(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Pauses the current song.
#[command]
async fn pause(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager = data
        .get::<SongbirdKey>()
        .expect("Expected Songbird in TypeMap");
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if queue.current().is_some() {
            let _ = queue.pause();
            msg.react(ctx, '✅').await?;
        } else {
            msg.reply(ctx, NOTHING_PLAYING).await?;
        }
    } else {
        msg.reply(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Resumes the current song.
#[command]
#[aliases(unpause)]
async fn resume(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager = data
        .get::<SongbirdKey>()
        .expect("Expected Songbird in TypeMap");
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if queue.current().is_some() {
            let _ = queue.resume();
            msg.react(ctx, '✅').await?;
        } else {
            msg.reply(ctx, NOTHING_PLAYING).await?;
        }
    } else {
        msg.reply(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Displays the information about the currently playing song.
#[command]
#[aliases(np, nowplaying, playing)]
async fn now_playing(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager = data
        .get::<SongbirdKey>()
        .expect("Expected Songbird in TypeMap");
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if let Some(np) = queue.current() {
            let metadata = np.metadata();
            msg.channel_id
                .send_message(ctx, |m| {
                    m.reference_message(msg);
                    get_now_playing_embed(m, metadata.clone());
                    m
                })
                .await?;
        } else {
            msg.reply(ctx, NOTHING_PLAYING).await?;
        }
    } else {
        msg.reply(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

// /// Change repeat mode.
// ///
// /// Usage: `repeat <one|all|off>`
// /// or `repeat one`
// #[command]
// #[num_args(1)]
// async fn repeat(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
//     let mode = args.message();
//     let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
//     let data = ctx.data.read().await;
//     let player_lock = data
//         .get::<PlayerManager>()
//         .cloned()
//         .expect("Expected PlayerManger in TypeMap");
//     let mut pm = player_lock.write().await;
//     if let Some(player) = pm.get_mut(&(guild_id.0 as u64)) {
//         match mode {
//             "one" => {
//                 player.set_repeat(Repeat::One);
//                 msg.react(ctx, '🔂').await?;
//             }
//             "all" => {
//                 player.set_repeat(Repeat::All);
//                 msg.react(ctx, '🔁').await?;
//             }
//             "off" => {
//                 player.set_repeat(Repeat::Off);
//                 msg.react(ctx, '✅').await?;
//             }
//             _ => {
//                 msg.reply(ctx, "Invalid repeat mode").await?;
//             }
//         }
//     } else {
//         msg.reply(ctx, JOIN_MSG).await?;
//     }

//     Ok(())
// }

/// Remove a song from queue.
///
/// Usage: `remove <index>`
/// or `remove 1`
#[command]
#[num_args(1)]
async fn remove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let index = args.single::<usize>().unwrap();
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager = data
        .get::<SongbirdKey>()
        .expect("Expected Songbird in TypeMap");
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if !queue.is_empty() {
            if let Some(t) = queue.dequeue(index) {
                msg.reply(
                    ctx,
                    format!("Removed - {}", t.metadata().title.clone().unwrap()),
                )
                .await?;
            } else {
                msg.reply(ctx, "Out of bounds").await?;
            }
        } else {
            msg.reply(ctx, QUEUE_EMPTY_MSG).await?;
        }
    } else {
        msg.reply(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

#[derive(Copy, Clone, Debug, EnumString, ToString)]
enum LofiStation {
    #[strum(serialize = "https://youtu.be/5qap5aO4i9A", serialize = "ChilledCow")]
    ChilledCow,
    #[strum(serialize = "https://youtu.be/DWcJFNfaw9c", serialize = "ChilledCow2")]
    ChilledCow2,
    #[strum(
        serialize = "https://youtu.be/5yx6BWlEVcY",
        serialize = "ChillHopMusic"
    )]
    ChillHopMusic,
    #[strum(
        serialize = "https://youtu.be/7NOSDKb0HlU",
        serialize = "ChillHopMusic2"
    )]
    ChillHopMusic2,
    #[strum(
        serialize = "https://youtu.be/WBfbkPTqUtU",
        serialize = "TokyoLostTracks"
    )]
    TokyoLostTracks,
    #[strum(
        serialize = "https://youtu.be/OVPPOwMpSpQ",
        serialize = "TheJazzhopCafe"
    )]
    TheJazzhopCafe,
    #[strum(
        serialize = "https://youtu.be/ZYMuB9y549s",
        serialize = "HomeworkRadio"
    )]
    HomeworkRadio,
    #[strum(serialize = "https://youtu.be/-5KAN9_CzSA", serialize = "SteezyAsFuck")]
    SteezyAsFuck,
    #[strum(
        serialize = "https://youtu.be/l7TxwBhtTUY",
        serialize = "TheBootLegBoy"
    )]
    TheBootLegBoy,
    #[strum(serialize = "https://youtu.be/B8tQ8RUbTW8", serialize = "InYourChill")]
    InYourChill,
    #[strum(serialize = "https://youtu.be/bM0Iw7PPoU4", serialize = "CollegeMusic")]
    CollegeMusic,
}

/// Play a lofi stream.
///
/// Usage: `lofi <id>`
///
/// Available Channels:
/// ```
/// +-----------------+------------------------------+
/// |       ID        |             URL              |
/// +-----------------+------------------------------+
/// | ChilledCow      | https://youtu.be/5qap5aO4i9A |
/// | ChilledCow2     | https://youtu.be/DWcJFNfaw9c |
/// | ChillHopMusic   | https://youtu.be/5yx6BWlEVcY |
/// | ChillHopMusic2  | https://youtu.be/7NOSDKb0HlU |
/// | TokyoLostTracks | https://youtu.be/WBfbkPTqUtU |
/// | TheJazzhopCafe  | https://youtu.be/OVPPOwMpSpQ |
/// | HomeworkRadio   | https://youtu.be/ZYMuB9y549s |
/// | SteezyAsFuck    | https://youtu.be/-5KAN9_CzSA |
/// | TheBootLegBoy   | https://youtu.be/l7TxwBhtTUY |
/// | InYourChill     | https://youtu.be/B8tQ8RUbTW8 |
/// | CollegeMusic    | https://youtu.be/bM0Iw7PPoU4 |
/// +-----------------+------------------------------+
/// ```
#[command]
#[num_args(1)]
async fn lofi(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    match args.single::<LofiStation>() {
        Ok(station) => {
            play(
                ctx,
                msg,
                Args::new(&station.to_string(), &[Delimiter::Single(' ')]),
            )
            .await?;
        }
        Err(_) => {
            msg.reply(
                ctx,
                "Invalid channel ID, try `help lofi` for all the available channels.",
            )
            .await?;
        }
    }

    Ok(())
}
