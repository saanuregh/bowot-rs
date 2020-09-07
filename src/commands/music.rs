use crate::{utils::spotify::get_spotify_tracks, Lavalink, VoiceGuildUpdate, VoiceManager};

use std::{sync::Arc, time::Duration};

use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::{channel::Message, misc::Mentionable},
    prelude::Context,
};

use lavalink_rs::{model::Track, LavalinkClient};

use regex::Regex;
use serde_json;

use failure::Error;
use failure::Fail;

use rand::seq::SliceRandom;
use rand::thread_rng;

//use tracing::{
//    //Log macros.
//    info,
//    trace,
//    debug,
//    warn,
//    error,
//};

#[derive(Debug, Fail)]
#[fail(display = "Not in a voice channel.")]
struct JoinError;

pub async fn _join(ctx: &Context, msg: &Message) -> Result<String, Error> {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            msg.reply(ctx, "You are not connected to a voice channel")
                .await?;

            return Err(JoinError.into());
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
    let has_joined = manager.join(guild_id, connect_to).is_some();

    if has_joined {
        drop(manager);

        loop {
            let data = ctx.data.read().await;
            let vgu_lock = data.get::<VoiceGuildUpdate>().unwrap();
            let mut vgu = vgu_lock.write().await;
            if !vgu.contains(&guild_id) {
                tokio::time::delay_for(Duration::from_millis(500)).await;
            } else {
                vgu.remove(&guild_id);
                break;
            }
        }

        let manager_lock = ctx
            .data
            .read()
            .await
            .get::<VoiceManager>()
            .cloned()
            .expect("Expected VoiceManager in TypeMap.");
        let manager = manager_lock.lock().await;

        let mut data = ctx.data.write().await;
        let lava_client_lock = data
            .get_mut::<Lavalink>()
            .expect("Expected a lavalink client in TypeMap");
        let handler = manager.get(guild_id).unwrap();
        lava_client_lock
            .lock()
            .await
            .create_session(guild_id, &handler)
            .await?;

        Ok(connect_to.mention())
    } else {
        msg.channel_id.say(ctx, "Error joining the channel").await?;
        Err(JoinError.into())
    }
}

/// Joins me to the voice channel you are currently on.
#[command]
#[aliases("connect")]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let channel = _join(ctx, msg).await?;
    msg.channel_id
        .say(ctx, &format!("Joined {}", channel))
        .await?;

    Ok(())
}

/// Shuffles the order of the current queue.
#[command]
#[aliases(randomize)]
async fn shuffle(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let lava_client_lock = data
        .get::<Lavalink>()
        .expect("Expected a lavalink client in TypeMap");
    let mut lava_client = lava_client_lock.lock().await;
    if let Some(node) = lava_client.nodes.get_mut(&msg.guild_id.unwrap().0) {
        {
            let mut rng = thread_rng();
            let mut queue = node.queue.clone();
            queue.shuffle(&mut rng);
            node.queue = queue.clone();
        }
        msg.react(ctx, '✅').await?;
    };

    Ok(())
}

/// Skips the current song being played.
///
/// NOTE: will not skip if there's no more songs in the queue.
/// Use `stop` or `pause` instad.
#[command]
#[aliases(next)]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let lava_client_lock = data
        .get::<Lavalink>()
        .expect("Expected a lavalink client in TypeMap");
    let mut lava_client = lava_client_lock.lock().await;

    if let Some(track) = lava_client.skip(msg.guild_id.unwrap()).await {
        let track_info = track.track.info.as_ref().unwrap();
        msg.channel_id
            .send_message(ctx, |m| {
                m.content("Skipped:");
                m.embed(|e| {
                    e.title(&track_info.title);
                    e.thumbnail(format!(
                        "https://i.ytimg.com/vi/{}/default.jpg",
                        &track_info.identifier
                    ));
                    e.url(&track_info.uri);
                    e.footer(|f| f.text(format!("Submited by unknown")));
                    e.field("Uploader", &track_info.author, true);
                    e.field(
                        "Length",
                        format!("{}:{}", track_info.length / 1000 % 3600 / 60, {
                            let x = track_info.length / 1000 % 3600 % 60;
                            if x < 10 {
                                format!("0{}", x)
                            } else {
                                x.to_string()
                            }
                        }),
                        true,
                    );
                    e
                })
            })
            .await?;
        let node = lava_client.nodes.get(&msg.guild_id.unwrap().0).unwrap();
        if node.queue.is_empty() && node.now_playing.is_none() {
            lava_client.stop(msg.guild_id.unwrap()).await?;
        }
    } else {
        msg.channel_id.say(ctx, "Nothing to skip.").await?;
    }

    Ok(())
}

/// Displays the current song queue.
#[command]
#[aliases(que)]
async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let lava_client_lock = data
        .get::<Lavalink>()
        .expect("Expected a lavalink client in TypeMap");
    let mut lava_client = lava_client_lock.lock().await;
    if let Some(node) = lava_client.nodes.get_mut(&msg.guild_id.unwrap().0) {
        if !node.queue.is_empty() {
            let mut queue = String::from("```st\n");
            for (index, track) in node.queue.iter().skip(1).take(10).enumerate() {
                queue += &format!(
                    "{}: {}\n",
                    index + 1,
                    track.track.info.as_ref().unwrap().title
                );
            }
            if node.queue.len() > 10 {
                queue += &format!("... {}", node.queue.len());
            }

            queue += "\n```";

            queue = queue.replace("@", "@\u{200B}");

            msg.channel_id.say(ctx, &queue).await?;
        } else {
            msg.channel_id.say(ctx, "The queue is empty.").await?;
        }
    };

    Ok(())
}

/// Clears the current queue.
#[command]
#[aliases(cque, clearqueue, clearque, cqueue)]
async fn clear_queue(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let lava_client_lock = data
        .get::<Lavalink>()
        .expect("Expected a lavalink client in TypeMap");
    let mut lava_client = lava_client_lock.lock().await;
    if let Some(node) = lava_client.nodes.get_mut(&msg.guild_id.unwrap().0) {
        if !node.queue.is_empty() {
            node.queue = vec![];

            msg.react(ctx, '✅').await?;
        } else {
            msg.channel_id
                .say(ctx, "The queue is already empty.")
                .await?;
        }
    };

    Ok(())
}

/// Displays the information about the currently playing song.
#[command]
#[aliases(np, nowplaying, playing)]
async fn now_playing(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let lava_client_lock = data
        .get::<Lavalink>()
        .expect("Expected a lavalink client in TypeMap");
    let lava_client = lava_client_lock.lock().await;

    if let Some(node) = lava_client.nodes.get(&msg.guild_id.unwrap().0) {
        let track = node.now_playing.as_ref();
        if let Some(x) = track {
            let track_info = x.track.info.as_ref().unwrap();
            msg.channel_id
                .send_message(ctx, |m| {
                    m.content("Now playing:");
                    m.embed(|e| {
                        e.title(&track_info.title);
                        e.thumbnail(format!(
                            "https://i.ytimg.com/vi/{}/default.jpg",
                            track_info.identifier
                        ));
                        e.url(&track_info.uri);
                        e.footer(|f| f.text(format!("Submited by unknown")));
                        e.field("Uploader", &track_info.author, true);
                        e.field(
                            "Length",
                            format!("{}:{}", track_info.length / 1000 % 3600 / 60, {
                                let x = track_info.length / 1000 % 3600 % 60;
                                if x < 10 {
                                    format!("0{}", x)
                                } else {
                                    x.to_string()
                                }
                            }),
                            true,
                        );
                        e
                    })
                })
                .await?;
        } else {
            msg.channel_id
                .say(ctx, "Nothing is playing at the moment.")
                .await?;
        }
    } else {
        msg.channel_id
            .say(ctx, "Nothing is playing at the moment.")
            .await?;
    }

    Ok(())
}

/// Jumps to the specific time in seconds to the currently playing song.
#[command]
#[min_args(1)]
#[aliases(jump_to, jumpto, scrub)]
async fn seek(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let num = if let Ok(x) = args.single::<u64>() {
        x
    } else {
        msg.reply(&ctx.http, "Provide a valid number of seconds.")
            .await?;
        return Ok(());
    };

    let data = ctx.data.read().await;
    let lava_client_lock = data
        .get::<Lavalink>()
        .expect("Expected a lavalink client in TypeMap");
    let mut lava_client = lava_client_lock.lock().await;

    lava_client
        .seek(msg.guild_id.unwrap(), Duration::from_secs(num))
        .await?;

    msg.react(ctx, '✅').await?;

    Ok(())
}

/// Stops the current player.
#[command]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let lava_client_lock = data
        .get::<Lavalink>()
        .expect("Expected a lavalink client in TypeMap");
    let mut lava_client = lava_client_lock.lock().await;

    lava_client.stop(msg.guild_id.unwrap()).await?;

    msg.react(ctx, '✅').await?;

    Ok(())
}

/// Pauses the current player.
#[command]
async fn pause(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let lava_client_lock = data
        .get::<Lavalink>()
        .expect("Expected a lavalink client in TypeMap");
    let mut lava_client = lava_client_lock.lock().await;

    lava_client.set_pause(msg.guild_id.unwrap(), true).await?;

    msg.react(ctx, '✅').await?;

    Ok(())
}

/// Resumes the current player.
#[command]
#[aliases(unpause)]
async fn resume(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let lava_client_lock = data
        .get::<Lavalink>()
        .expect("Expected a lavalink client in TypeMap");
    let mut lava_client = lava_client_lock.lock().await;

    lava_client.set_pause(msg.guild_id.unwrap(), false).await?;

    msg.react(ctx, '✅').await?;

    Ok(())
}

/// Disconnects me from the voice channel if im in one.
#[command]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = ctx
        .cache
        .guild_channel_field(msg.channel_id, |channel| channel.guild_id)
        .await
        .unwrap();

    let manager_lock = ctx
        .data
        .read()
        .await
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in TypeMap.");
    let mut manager = manager_lock.lock().await;
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        manager.remove(guild_id);

        let mut data = ctx.data.write().await;
        let lava_client_lock = data
            .get_mut::<Lavalink>()
            .expect("Expected a lavalink client in TypeMap");
        lava_client_lock.lock().await.destroy(guild_id).await?;

        msg.react(ctx, '✅').await?;
    } else {
        msg.reply(ctx, "Not in a voice channel").await?;
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

    let guild_id = match ctx.cache.guild_channel(msg.channel_id).await {
        Some(channel) => channel.guild_id,
        None => {
            msg.channel_id
                .say(ctx, "Error finding channel info")
                .await?;

            return Ok(());
        }
    };

    let manager_lock = ctx
        .data
        .read()
        .await
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in ShareMap.");
    let mut manager = manager_lock.lock().await;

    if let Some(_handler) = manager.get_mut(guild_id) {
        let data = ctx.data.read().await;
        let lava_lock = data
            .get::<Lavalink>()
            .expect("Expected a lavalink client in TypeMap");
        let lava_client = lava_lock.lock().await;

        let mut iter = 0;
        let query_information = loop {
            iter += 1;
            let res = lava_client.auto_search_tracks(&query).await?;

            if res.tracks.is_empty() {
                if iter == 5 {
                    msg.channel_id
                        .say(&ctx, "Could not find any video of the search query.")
                        .await?;
                    return Ok(());
                }
            } else {
                if query.starts_with("http") && res.tracks.len() > 1 {
                    msg.channel_id.say(ctx, "If you would like to play the entire playlist, use `play_playlist` instead.").await?;
                }
                break res;
            }
        };

        drop(lava_client);

        LavalinkClient::play(guild_id, query_information.tracks[0].clone())
            .queue(Arc::clone(lava_lock))
            .await?;

        msg.channel_id
            .send_message(ctx, |m| {
                m.content("Added to queue:");
                m.embed(|e| {
                    e.title(&query_information.tracks[0].info.as_ref().unwrap().title);
                    e.thumbnail(format!(
                        "https://i.ytimg.com/vi/{}/default.jpg",
                        query_information.tracks[0]
                            .info
                            .as_ref()
                            .unwrap()
                            .identifier
                    ));
                    e.url(&query_information.tracks[0].info.as_ref().unwrap().uri);
                    e.footer(|f| f.text(format!("Submited by {}", &msg.author.name)));
                    e.field(
                        "Uploader",
                        &query_information.tracks[0].info.as_ref().unwrap().author,
                        true,
                    );
                    e.field(
                        "Length",
                        format!(
                            "{}:{}",
                            query_information.tracks[0].info.as_ref().unwrap().length / 1000 % 3600
                                / 60,
                            {
                                let x = query_information.tracks[0].info.as_ref().unwrap().length
                                    / 1000
                                    % 3600
                                    % 60;
                                if x < 10 {
                                    format!("0{}", x)
                                } else {
                                    x.to_string()
                                }
                            }
                        ),
                        true,
                    );
                    e
                })
            })
            .await?;
    } else {
        msg.channel_id.say(ctx, "Please, connect the bot to the voice channel you are currently on first with the `join` command.").await?;
    }

    Ok(())
}

/// Adds an entire playlist to the queue.
///
/// Usage: `play https://www.youtube.com/playlist?list=PLTktV6LgA75yif8RR7yUiSttZD7GKtl_5`
#[command]
#[min_args(1)]
#[aliases(playlist, playplaylist, play_list, pl, playl, plist)]
async fn play_playlist(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
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

    let guild_id = match ctx.cache.guild_channel(msg.channel_id).await {
        Some(channel) => channel.guild_id,
        None => {
            msg.channel_id
                .say(ctx, "Error finding channel info")
                .await?;

            return Ok(());
        }
    };

    let manager_lock = ctx
        .data
        .read()
        .await
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in ShareMap.");
    let mut manager = manager_lock.lock().await;

    if let Some(_handler) = manager.get_mut(guild_id) {
        let data = ctx.data.read().await;
        let lava_lock = data
            .get::<Lavalink>()
            .expect("Expected a lavalink client in TypeMap");
        let lava_client = lava_lock.lock().await;

        let mut iter = 0;
        let query_information = loop {
            iter += 1;
            let res = lava_client.auto_search_tracks(&query).await?;

            if res.tracks.is_empty() {
                if iter == 5 {
                    msg.channel_id
                        .say(&ctx, "Could not find any video of the search query.")
                        .await?;
                    return Ok(());
                }
            } else {
                break res;
            }
        };

        drop(lava_client);

        for track in query_information.clone().tracks {
            LavalinkClient::play(guild_id, track.clone())
                .queue(Arc::clone(lava_lock))
                .await?;
        }

        msg.channel_id
            .send_message(ctx, |m| {
                m.content("Added playlist to queue.");
                m.embed(|e| {
                    e.title(
                        query_information
                            .playlist_info
                            .name
                            .unwrap_or("Playlist".to_string()),
                    );
                    e.url(query);
                    e.footer(|f| f.text(format!("Submited by {}", &msg.author.name)))
                })
            })
            .await?;
    } else {
        msg.channel_id.say(ctx, "Please, connect the bot to the voice channel you are currently on first with the `join` command.").await?;
    }

    Ok(())
}

/// Adds an entire spotify playlist to the queue.
///
/// Usage: `play_spotify https://open.spotify.com/playlist/1uoJah7SRmx6wFVXKsyynB?si=pnl8uHdoTryEfj_k0yyckA`
#[command]
#[min_args(1)]
#[aliases(spotify)]
async fn play_spotify(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut embeded = false;
    let mut query = args.message().to_string();
    if query.starts_with('<') && query.ends_with('>') {
        embeded = true;
        let re = Regex::new("[<>]").unwrap();
        query = re.replace_all(&query, "").into_owned();
    }
    if !query.starts_with("http") {
        msg.reply(ctx, "Provide a valid URL").await?;
        return Ok(());
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
            msg.channel_id
                .say(ctx, "Please, put the url between <> so it doesn't embed.")
                .await?;
        }
    }

    let guild_id = match ctx.cache.guild_channel(msg.channel_id).await {
        Some(channel) => channel.guild_id,
        None => {
            msg.channel_id
                .say(ctx, "Error finding channel info")
                .await?;

            return Ok(());
        }
    };

    let manager_lock = ctx
        .data
        .read()
        .await
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in ShareMap.");
    let mut manager = manager_lock.lock().await;

    if let Some(_handler) = manager.get_mut(guild_id) {
        let data = ctx.data.read().await;
        let lava_lock = data
            .get::<Lavalink>()
            .expect("Expected a lavalink client in TypeMap");
        let lava_client = lava_lock.lock().await;
        let mut tracks: Vec<Track> = Vec::new();
        let (playlist_name, spotify_tracks) = get_spotify_tracks(query.clone())
            .await
            .expect("Couldn't get tracks from spotify");
        {
            for spotify_track in spotify_tracks {
                let query_information = lava_client.auto_search_tracks(spotify_track).await?;
                if !query_information.tracks.is_empty() {
                    tracks.push(query_information.tracks[0].clone());
                }
            }
        }

        drop(lava_client);

        if tracks.is_empty() {
            msg.channel_id
                .say(
                    ctx,
                    "Empty playlist or couldn't obtain playlist items successfully.",
                )
                .await?;
            return Ok(());
        }

        for track in tracks {
            LavalinkClient::play(guild_id, track.clone())
                .queue(Arc::clone(lava_lock))
                .await?;
        }

        msg.channel_id
            .send_message(ctx, |m| {
                m.content("Added playlist to queue.");
                m.embed(|e| {
                    e.title(playlist_name);
                    e.url(query);
                    e.footer(|f| f.text(format!("Submited by {}", &msg.author.name)))
                })
            })
            .await?;
    } else {
        msg.channel_id.say(ctx, "Please, connect the bot to the voice channel you are currently on first with the `join` command.").await?;
    }

    Ok(())
}
