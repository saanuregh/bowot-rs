use crate::{
    player::{Player, Repeat, Track},
    PlayerManager, VoiceManager,
};
use regex::Regex;
use serde_json;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::{channel::Message, misc::Mentionable},
    prelude::Context,
    voice,
};
use std::{sync::Arc, time::Duration};
use tracing::error;
use youtube_dl::YoutubeDl;

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
    let manager_lock = ctx
        .data
        .read()
        .await
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let mut manager = manager_lock.lock().await;
    if manager.join(guild_id, connect_to).is_some() {
        let data = ctx.data.read().await;
        let pm_lock = data
            .get::<PlayerManager>()
            .cloned()
            .expect("Expected PlayerManager in TypeMap");
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
    match _join(ctx, msg).await {
        Some(_) => {
            msg.react(ctx, '✅').await?;
        }
        None => {
            msg.channel_id.say(ctx, "Not in a voice channel").await?;
        }
    }

    Ok(())
}

/// Disconnects me from the voice channel if im in one.
#[command]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let manager_lock = ctx
        .data
        .read()
        .await
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let mut manager = manager_lock.lock().await;
    if manager.get(guild_id).is_some() {
        manager.remove(guild_id);
        msg.react(ctx, '✅').await?;
    } else {
        msg.channel_id.say(ctx, "Please, connect the bot to the voice channel you are currently on first with the `join` command.").await?;
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
    let player = pm
        .get(&(guild_id.0 as u64))
        .expect("No player for this guild available");
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
        msg.channel_id.say(ctx, "The queue is empty").await?;
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
    let player = pm
        .get_mut(&(guild_id.0 as u64))
        .expect("No player for this guild available");
    if !player.clone().is_empty() {
        player.clear_except_np();
        msg.react(ctx, '✅').await?;
    } else {
        msg.channel_id.say(ctx, "The queue is empty").await?;
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
    let player = pm
        .get_mut(&(guild_id.0 as u64))
        .expect("No player for this guild available");
    if !player.clone().is_empty() {
        player.shuffle();
        msg.react(ctx, '✅').await?;
    } else {
        msg.channel_id.say(ctx, "The queue is empty").await?;
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
    let manager_lock = ctx
        .data
        .read()
        .await
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in ShareMap.");
    let manager = manager_lock.lock().await;
    let has_joined = manager.get(guild_id).is_some();
    if !has_joined {
        drop(manager);
        _join(ctx, msg).await;
    }
    let data = ctx.data.read().await;
    let player_lock = data
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected Player Manger in TypeMap");
    let mut pm = player_lock.write().await;
    let player = pm
        .get_mut(&(guild_id.0 as u64))
        .expect("No player for this guild available");
    match YoutubeDl::new(query).run()? {
        youtube_dl::YoutubeDlOutput::Playlist(p) => {
            if let Some(playlist) = p.entries {
                if playlist.is_empty() {
                    msg.channel_id.say(ctx, "No result.").await?;
                    return Ok(());
                }
                for s in playlist.clone().into_iter() {
                    let track = Track {
                        url: format!("https://www.youtube.com/watch?v={}", s.id),
                        title: s.title,
                        requester: msg.author.id,
                        live: s.is_live.unwrap_or(false),
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
            };
            player.add_track(track);
            msg.channel_id
                .say(ctx, format!("Queued - {}", s.title))
                .await?;
        }
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
        let player = pm
            .get_mut(&(guild_id.0 as u64))
            .expect("No player for this guild available");
        if player.clone().is_empty() {
            if let Err(why) = msg.channel_id.say(ctx.clone(), "Stopping playback").await {
                error!("Player Worker: {:?}", why)
            }
            break;
        }
        let now_playing = player.clone().queue.first().cloned().unwrap();
        if let Err(why) = msg
            .channel_id
            .send_message(ctx.clone(), |m| {
                m.content("Now playing:");
                m.embed(|e| {
                    e.title(&now_playing.title);
                    e.url(&now_playing.url);
                    e.field("Requester", now_playing.requester.mention(), true);
                    e.field("Live", now_playing.live, true);
                    e
                })
            })
            .await
        {
            error!("Player Worker: {:?}", why)
        }
        let source = match voice::ytdl(now_playing.url.as_str()).await {
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
            let player_2 = pm_2
                .get_mut(&(guild_id.0 as u64))
                .expect("No player for this guild available");
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
        }
    }
}

/// Skips the current song being played.
#[command]
#[aliases(next)]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let manager_lock = ctx
        .data
        .read()
        .await
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let mut manager = manager_lock.lock().await;
    match manager.get_mut(guild_id) {
        Some(handler) => {
            let data = ctx.data.read().await;
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
                msg.channel_id.say(ctx, "Nothing playing").await?;
            }
        }
        None => {
            msg.channel_id.say(ctx, "Please, connect the bot to the voice channel you are currently on first with the `join` command.").await?;
        }
    }

    Ok(())
}

/// Pauses the current song.
#[command]
async fn pause(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let manager_lock = ctx
        .data
        .read()
        .await
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let mut manager = manager_lock.lock().await;
    match manager.get_mut(guild_id) {
        Some(_) => {
            let data = ctx.data.read().await;
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
                msg.channel_id.say(ctx, "Nothing playing").await?;
            }
        }
        None => {
            msg.channel_id.say(ctx, "Please, connect the bot to the voice channel you are currently on first with the `join` command.").await?;
        }
    }

    Ok(())
}

/// Resumes the current song.
#[command]
#[aliases(unpause)]
async fn resume(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let manager_lock = ctx
        .data
        .read()
        .await
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let mut manager = manager_lock.lock().await;
    match manager.get_mut(guild_id) {
        Some(_) => {
            let data = ctx.data.read().await;
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
                msg.channel_id.say(ctx, "Nothing playing").await?;
            }
        }
        None => {
            msg.channel_id.say(ctx, "Please, connect the bot to the voice channel you are currently on first with the `join` command.").await?;
        }
    }

    Ok(())
}

/// Stops the current player (clears song queue).
#[command]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild(&ctx.cache).await.unwrap().id;
    let manager_lock = ctx
        .data
        .read()
        .await
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let mut manager = manager_lock.lock().await;
    match manager.get_mut(guild_id) {
        Some(handler) => {
            let data = ctx.data.read().await;
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
                msg.channel_id.say(ctx, "Nothing playing").await?;
            }
        }
        None => {
            msg.channel_id.say(ctx, "Please, connect the bot to the voice channel you are currently on first with the `join` command.").await?;
        }
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
    let player = pm
        .get_mut(&(guild_id.0 as u64))
        .expect("No player for this guild available");
    if !player.is_finished().await {
        let now_playing = player.clone().queue.first().cloned().unwrap();
        msg.channel_id
            .send_message(ctx.clone(), |m| {
                m.content("Now playing:");
                m.embed(|e| {
                    e.title(&now_playing.title);
                    e.url(&now_playing.url);
                    e.field("Requester", now_playing.requester.mention(), true);
                    e.field("Live", now_playing.live, true);
                    e
                })
            })
            .await?;
    } else {
        msg.channel_id.say(ctx, "Nothing playing").await?;
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
    let player = pm
        .get_mut(&(guild_id.0 as u64))
        .expect("No player for this guild available");
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
    let player = pm
        .get_mut(&(guild_id.0 as u64))
        .expect("No player for this guild available");
    if !player.clone().is_empty() {
        match player.remove_track(index) {
            Some(t) => {
                msg.channel_id
                    .say(ctx, format!("Removed - {}", t.title))
                    .await?;
            }
            None => {
                msg.channel_id.say(ctx, "Out of bounds").await?;
            }
        }
    } else {
        msg.channel_id.say(ctx, "The queue is empty").await?;
    }

    Ok(())
}
