use crate::{
    player::{Player, Repeat, Track},
    PlayerManager, VoiceManager,
};
use regex::Regex;
use serde_json;
use serenity::{
    builder::CreateMessage,
    framework::standard::{macros::command, Args, CommandResult},
    model::{channel::Message, misc::Mentionable},
    prelude::Context,
    voice,
};
use std::{sync::Arc, time::Duration};
use tracing::error;
use youtube_dl::YoutubeDl;

const JOIN_MSG: &str = "Please, connect the bot to the voice channel you are currently on first with the `join` command.";
const QUEUE_EMPTY_MSG: &str = "The queue is empty";
const NOTIN_VC_MSG: &str = "Not in a voice channel";
const NOTHING_PLAYING: &str = "Nothing playing";

pub async fn _join(ctx: &Context, msg: &Message) -> Option<String> {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;
    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            return None;
        }
    };
    let data = ctx.data.read().await;
    let manager_lock = data
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let pm_lock = data
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected PlayerManager in TypeMap");
    let mut manager = manager_lock.lock().await;
    if manager.join(guild_id, connect_to).is_some() {
        let mut pm = pm_lock.write().await;
        pm.insert(guild_id.0 as u64, Player::new(guild_id));
        Some(connect_to.mention())
    } else {
        None
    }
}

/// Joins me to the voice channel you are currently on.
#[command]
#[aliases("connect")]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    if _join(ctx, msg).await.is_some() {
        msg.react(ctx, '✅').await?;
    } else {
        msg.channel_id.say(ctx, NOTIN_VC_MSG).await?;
    }

    Ok(())
}

/// Disconnects me from the voice channel if im in one.
#[command]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager_lock = data
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let mut manager = manager_lock.lock().await;
    if let Some(handler) = manager.get_mut(guild_id) {
        let pm_lock = data
            .get::<PlayerManager>()
            .cloned()
            .expect("Expected PlayerManager in TypeMap");
        let mut pm = pm_lock.write().await;
        if let Some(player) = pm.get_mut(&(guild_id.0 as u64)) {
            if !player.is_finished().await {
                player.reset();
                handler.stop();
            }
            pm.remove(&(guild_id.0 as u64));
        }
        manager.remove(guild_id);
        msg.react(ctx, '✅').await?;
    } else {
        msg.channel_id.say(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Show the song queue.
#[command]
async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let player_lock = ctx
        .data
        .read()
        .await
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected PlayerManger in TypeMap");
    let pm = player_lock.read().await;
    if let Some(player) = pm.get(&(guild_id.0 as u64)) {
        let queue = player.clone().queue;
        if !queue.is_empty() {
            let mut queue_str = String::from("```st\n");
            queue_str += &format!("Now playing: {}\n", queue[0].title);
            for (index, track) in queue[1..].iter().take(10).enumerate() {
                queue_str += &format!("{}: {}\n", index + 1, track.title);
            }
            if queue.len() > 10 {
                queue_str += &format!("... {}", queue.len());
            }
            queue_str += "\n```";
            queue_str = queue_str.replace("@", "@\u{200B}");
            msg.channel_id.say(ctx, &queue_str).await?;
        } else {
            msg.channel_id.say(ctx, QUEUE_EMPTY_MSG).await?;
        }
    } else {
        msg.channel_id.say(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Clears the song queue.
#[command]
async fn clear_queue(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let player_lock = data
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected PlayerManger in TypeMap");
    let mut pm = player_lock.write().await;
    if let Some(player) = pm.get_mut(&(guild_id.0 as u64)) {
        if !player.clone().is_empty() {
            player.clear_except_np();
            msg.react(ctx, '✅').await?;
        } else {
            msg.channel_id.say(ctx, QUEUE_EMPTY_MSG).await?;
        }
    } else {
        msg.channel_id.say(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Shuffles the song queue.
#[command]
async fn shuffle(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let player_lock = data
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected PlayerManger in TypeMap");
    let mut pm = player_lock.write().await;
    if let Some(player) = pm.get_mut(&(guild_id.0 as u64)) {
        if !player.clone().is_empty() {
            player.shuffle();
            msg.react(ctx, '✅').await?;
        } else {
            msg.channel_id.say(ctx, QUEUE_EMPTY_MSG).await?;
        }
    } else {
        msg.channel_id.say(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Adds a song to the queue.
///
/// Usage: `play starmachine2000`
/// or `play https://www.youtube.com/watch?v=dQw4w9WgXcQ`
#[command]
#[min_args(1)]
#[aliases(p)]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut embeded = false;
    let mut query = args.message().to_string();
    if query.starts_with('<') && query.ends_with('>') {
        embeded = true;
        let re = Regex::new("[<>]").unwrap();
        query = re.replace_all(&query, "").into_owned();
    }
    if !embeded {
        if let Err(_) = ctx
            .http
            .edit_message(
                msg.channel_id.0,
                msg.id.0,
                &serde_json::json!({"flags" : 4}),
            )
            .await
        {
            if query.starts_with("http") {
                msg.channel_id
                    .say(ctx, "Please, put the url between <> so it doesn't embed.")
                    .await?;
            }
        }
    }
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager_lock = data
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in ShareMap.");
    let manager = manager_lock.lock().await;
    let has_joined = manager.get(guild_id).is_some();
    if !has_joined {
        drop(manager);
        if _join(ctx, msg).await.is_none() {
            msg.channel_id.say(ctx, NOTIN_VC_MSG).await?;
            return Ok(());
        }
    }

    let player_lock = data
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected Player Manger in TypeMap");
    let mut pm = player_lock.write().await;
    let player = pm
        .get_mut(&(guild_id.0 as u64))
        .expect("No player for this guild available");
    if let Ok(result) = YoutubeDl::new(query).run() {
        match result {
            youtube_dl::YoutubeDlOutput::Playlist(p) => {
                if let Some(playlist) = p.entries {
                    if playlist.is_empty() {
                        msg.channel_id
                            .say(ctx, "Couldn't find any result for the query")
                            .await?;
                        return Ok(());
                    }
                    for s in playlist.clone().into_iter() {
                        let track = Track {
                            url: format!("https://www.youtube.com/watch?v={}", s.id),
                            title: s.title,
                            requester: msg.author.id,
                            live: s.is_live.unwrap_or(false),
                            thumbnail: s.thumbnail,
                        };
                        player.add_track(track);
                    }
                    if playlist.len() > 1 {
                        msg.channel_id
                            .say(ctx, format!("Queued {} tracks", playlist.len()))
                            .await?;
                    } else {
                        msg.channel_id
                            .say(ctx, format!("Queued - {}", playlist.first().unwrap().title))
                            .await?;
                    }
                }
            }
            youtube_dl::YoutubeDlOutput::SingleVideo(s) => {
                let track = Track {
                    url: s.webpage_url.unwrap(),
                    title: s.title.clone(),
                    requester: msg.author.id,
                    live: s.is_live.unwrap_or(false),
                    thumbnail: s.thumbnail,
                };
                player.add_track(track);
                msg.channel_id
                    .say(ctx, format!("Queued - {}", s.title))
                    .await?;
            }
        }
    } else {
        msg.channel_id
            .say(ctx, "Couldn't find any result for the query")
            .await?;
        return Ok(());
    }
    if player.is_finished().await {
        let ctx_arc = Arc::new(ctx.clone());
        let msg_arc = Arc::new(msg.clone());
        tokio::spawn(async move {
            _player_worker(ctx_arc, msg_arc).await;
        });
    }

    Ok(())
}

async fn _player_worker(ctx: Arc<Context>, msg: Arc<Message>) {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    loop {
        let data = ctx.data.read().await;
        let player_lock = data
            .get::<PlayerManager>()
            .cloned()
            .expect("Expected Player Manger in TypeMap");
        let mut pm = player_lock.write().await;
        if let Some(player) = pm.get_mut(&(guild_id.0 as u64)) {
            if player.clone().is_empty() {
                if let Err(why) = msg.channel_id.say(ctx.clone(), "Queue finished").await {
                    error!("Player Worker: {:?}", why)
                }
                break;
            }
            let now_playing = player.clone().queue.first().cloned().unwrap();
            if let Err(why) = msg
                .channel_id
                .send_message(ctx.clone(), |m| {
                    _now_playing_embed(m, now_playing.clone());
                    m
                })
                .await
            {
                error!("Player Worker: {:?}", why)
            }
            let source = match voice::ytdl(&now_playing.url).await {
                Ok(source) => source,
                Err(_) => {
                    continue;
                }
            };
            let manager_lock = data
                .get::<VoiceManager>()
                .cloned()
                .expect("Expected VoiceManager in ShareMap.");
            let mut manager = manager_lock.lock().await;
            let handler = manager.get_mut(guild_id).unwrap();
            let now_source = handler.play_only(source);
            player.set_now_source(now_source);
            drop(manager);
            drop(pm);
            loop {
                let player_lock_2 = data
                    .get::<PlayerManager>()
                    .cloned()
                    .expect("Expected Player Manger in TypeMap");
                let mut pm_2 = player_lock_2.write().await;
                if let Some(player_2) = pm_2.get_mut(&(guild_id.0 as u64)) {
                    if player_2.is_finished().await {
                        match player_2.repeat {
                            Repeat::Off => {
                                player_2.pop();
                            }
                            Repeat::One => {}
                            Repeat::All => {
                                if let Some(t) = player_2.pop() {
                                    player_2.push(t);
                                }
                            }
                        }

                        break;
                    }
                    drop(pm_2);
                    tokio::time::delay_for(Duration::from_millis(500)).await;
                } else {
                    break;
                }
            }
        } else {
            break;
        }
    }
}

/// Skips the current song being played.
#[command]
#[aliases(next)]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager_lock = data
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let mut manager = manager_lock.lock().await;
    if let Some(handler) = manager.get_mut(guild_id) {
        let player_lock = data
            .get::<PlayerManager>()
            .cloned()
            .expect("Expected PlayerManger in TypeMap");
        let mut pm = player_lock.write().await;
        let player = pm
            .get_mut(&(guild_id.0 as u64))
            .expect("No player for this guild available");
        if !player.is_finished().await {
            player.reset();
            handler.stop();
            msg.react(ctx, '✅').await?;
        } else {
            msg.channel_id.say(ctx, NOTHING_PLAYING).await?;
        }
    } else {
        msg.channel_id.say(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Pauses the current song.
#[command]
async fn pause(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager_lock = data
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let manager = manager_lock.lock().await;
    if manager.get(guild_id).is_none() {
        msg.channel_id.say(ctx, JOIN_MSG).await?;
        return Ok(());
    }
    let player_lock = data
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected Player    Manger in TypeMap");
    let mut pm = player_lock.write().await;
    let player = pm
        .get_mut(&(guild_id.0 as u64))
        .expect("No player for this guild available");
    if !player.is_finished().await {
        if player.is_paused().await {
            msg.channel_id.say(ctx, "Already paused").await?;
        } else {
            player.pause().await;
            msg.react(ctx, '✅').await?;
        }
    } else {
        msg.channel_id.say(ctx, NOTHING_PLAYING).await?;
    }

    Ok(())
}

/// Resumes the current song.
#[command]
#[aliases(unpause)]
async fn resume(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager_lock = data
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let manager = manager_lock.lock().await;
    if manager.get(guild_id).is_none() {
        msg.channel_id.say(ctx, JOIN_MSG).await?;
        return Ok(());
    }
    let player_lock = data
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected Player    Manger in TypeMap");
    let mut pm = player_lock.write().await;
    let player = pm
        .get_mut(&(guild_id.0 as u64))
        .expect("No player for this guild available");
    if !player.is_finished().await {
        if !player.is_paused().await {
            msg.channel_id.say(ctx, "Already playing").await?;
        } else {
            player.play().await;
            msg.react(ctx, '✅').await?;
        }
    } else {
        msg.channel_id.say(ctx, NOTHING_PLAYING).await?;
    }

    Ok(())
}

/// Stops the current player (clears song queue).
#[command]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let manager_lock = data
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let mut manager = manager_lock.lock().await;
    if let Some(handler) = manager.get_mut(guild_id) {
        let player_lock = data
            .get::<PlayerManager>()
            .cloned()
            .expect("Expected PlayerManger in TypeMap");
        let mut pm = player_lock.write().await;
        let player = pm
            .get_mut(&(guild_id.0 as u64))
            .expect("No player for this guild available");
        if !player.is_finished().await {
            player.clear();
            player.reset();
            handler.stop();
            msg.react(ctx, '✅').await?;
        } else {
            msg.channel_id.say(ctx, NOTHING_PLAYING).await?;
        }
    } else {
        msg.channel_id.say(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Displays the information about the currently playing song.
#[command]
#[aliases(np, nowplaying, playing)]
async fn now_playing(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let player_lock = data
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected PlayerManger in TypeMap");
    let mut pm = player_lock.write().await;
    if let Some(player) = pm.get_mut(&(guild_id.0 as u64)) {
        if !player.is_finished().await {
            let now_playing = player.clone().queue.first().cloned().unwrap();
            msg.channel_id
                .send_message(ctx.clone(), |m| {
                    _now_playing_embed(m, now_playing);
                    m
                })
                .await?;
        } else {
            msg.channel_id.say(ctx, NOTHING_PLAYING).await?;
        }
    } else {
        msg.channel_id.say(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

/// Change repeat mode.
///
/// Usage: `repeat <one|all|off>`
/// or `repeat one`
#[command]
#[num_args(1)]
async fn repeat(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mode = args.message();
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let data = ctx.data.read().await;
    let player_lock = data
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected PlayerManger in TypeMap");
    let mut pm = player_lock.write().await;
    if let Some(player) = pm.get_mut(&(guild_id.0 as u64)) {
        match mode {
            "one" => {
                player.set_repeat(Repeat::One);
                msg.react(ctx, '🔂').await?;
            }
            "all" => {
                player.set_repeat(Repeat::All);
                msg.react(ctx, '🔁').await?;
            }
            "off" => {
                player.set_repeat(Repeat::Off);
                msg.react(ctx, '✅').await?;
            }
            _ => {
                msg.channel_id.say(ctx, "Invalid repeat mode").await?;
            }
        }
    } else {
        msg.channel_id.say(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

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
    let player_lock = data
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected PlayerManger in TypeMap");
    let mut pm = player_lock.write().await;
    if let Some(player) = pm.get_mut(&(guild_id.0 as u64)) {
        if !player.clone().is_empty() {
            if let Some(t) = player.remove_track(index) {
                msg.channel_id
                    .say(ctx, format!("Removed - {}", t.title))
                    .await?;
            } else {
                msg.channel_id.say(ctx, "Out of bounds").await?;
            }
        } else {
            msg.channel_id.say(ctx, QUEUE_EMPTY_MSG).await?;
        }
    } else {
        msg.channel_id.say(ctx, JOIN_MSG).await?;
    }

    Ok(())
}

pub fn _now_playing_embed(m: &mut CreateMessage, np: Track) {
    m.embed(|e| {
        e.title("Now playing");
        e.field("Title", &np.title, false);
        e.field("URL", &np.url, false);
        let live = match np.live {
            true => "Yes",
            false => "No",
        };
        e.field("Live", live, true);
        e.field("Requester", np.requester.mention(), true);
        if np.thumbnail.is_some() {
            e.thumbnail(np.thumbnail.clone().unwrap());
        }
        e
    });
}
