use crate::utils::basic_functions::{format_seconds, shorten};
use rand::Rng;
use serenity::{
    async_trait,
    builder::CreateMessage,
    client::Context,
    framework::standard::{macros::command, Args, CommandResult, Delimiter},
    http::Http,
    model::{channel::Message, id::GuildId, prelude::ChannelId},
    prelude::Mutex,
};

use std::{
    sync::atomic::Ordering,
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};
use tracing::error;
use youtube_dl::{YoutubeDl, YoutubeDlOutput};

use songbird::{
    input::Metadata,
    input::{ytdl, Input},
    Call, Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent,
};

const JOIN_MSG: &str = "Please, connect the bot to the voice channel you are currently on first with the `join` command.";
const QUEUE_EMPTY_MSG: &str = "The queue is empty";
const NOTIN_VC_MSG: &str = "Not in a voice channel";
const NOTHING_PLAYING: &str = "Nothing playing";
const MAX_PLAYLIST: usize = 25;

struct TrackEndNotifier {
    chan_id: ChannelId,
    http: Arc<Http>,
    handler_lock: Arc<Mutex<Call>>,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(_track_list) = ctx {
            let handler = self.handler_lock.lock().await;
            if let Some(np) = handler.queue().current() {
                let metadata = np.metadata();
                if let Err(why) = self
                    .chan_id
                    .send_message(&self.http, |m| {
                        _now_playing_embed(m, metadata.as_ref().clone());
                        m
                    })
                    .await
                {
                    error!("Error sending message: {:?}", why);
                }
            } else {
                if let Err(why) = self.chan_id.say(&self.http, "Queue finished").await {
                    error!("Error sending message: {:?}", why);
                }
            }
        }

        None
    }
}

struct ChannelIdleChecker {
    ctx: Arc<Context>,
    guild_id: GuildId,
    elapsed: Arc<AtomicUsize>,
}

#[async_trait]
impl VoiceEventHandler for ChannelIdleChecker {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let manager = songbird::get(self.ctx.clone().as_ref())
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();
        if let Some(handler_lock) = manager.get(self.guild_id) {
            let queue_empty = {
                let handler = handler_lock.lock().await;
                handler.queue().is_empty()
            };
            if queue_empty {
                if (self.elapsed.fetch_add(1, Ordering::Relaxed) + 1) >= 1 {
                    let _ = manager.remove(self.guild_id).await;
                    return Some(Event::Cancel);
                }
            } else {
                self.elapsed.store(0, Ordering::Relaxed);
                return None;
            }
        }

        Some(Event::Cancel)
    }
}

async fn _join(ctx: &Context, msg: &Message) -> Option<Arc<Mutex<Call>>> {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    if let Some(connect_to) = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
    {
        let manager = songbird::get(ctx)
            .await
            .expect("Songbird Voice client placed in at initialisation.")
            .clone();
        let (handler_lock, success) = manager.join(guild.id, connect_to).await;
        if let Err(why) = success {
            error!("Error while joining voice channel: {:?}", why);

            return None;
        }
        {
            let mut handler = handler_lock.lock().await;
            handler.add_global_event(
                Event::Track(TrackEvent::End),
                TrackEndNotifier {
                    chan_id: msg.channel_id,
                    http: ctx.http.clone(),
                    handler_lock: handler_lock.clone(),
                },
            );
            handler.add_global_event(
                Event::Periodic(Duration::from_secs(60), None),
                ChannelIdleChecker {
                    ctx: Arc::new(ctx.clone()),
                    guild_id: guild.id,
                    elapsed: Default::default(),
                },
            );
        }

        return Some(handler_lock);
    }

    None
}

/// Joins me to the voice channel you are currently on.
#[command]
#[aliases("connect")]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    if _join(ctx, msg).await.is_some() {
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
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
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
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    let handler_lock = match manager.get(guild_id) {
        Some(hl) => hl,
        None => match _join(ctx, msg).await {
            Some(hl) => hl,
            None => {
                msg.reply(ctx, NOTIN_VC_MSG).await?;

                return Ok(());
            }
        },
    };
    let loading_msg = msg.reply(ctx, "Loading...").await?;
    let mut sources: Vec<Input> = Vec::new();
    if let Ok(result) = YoutubeDl::new(query).run().await {
        match result {
            YoutubeDlOutput::Playlist(p) => {
                if let Some(playlist) = p.entries {
                    for s in playlist.clone().into_iter().take(MAX_PLAYLIST) {
                        match ytdl(&format!("https://www.youtube.com/watch?v={}", s.id)).await {
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
        let metadata = sources.first().unwrap().metadata.clone();
        if handler.queue().current().is_none() {
            msg.channel_id
                .send_message(ctx, |m| {
                    m.reference_message(msg);
                    _now_playing_embed(m, metadata);
                    m
                })
                .await?;
        } else {
            msg.reply(
                ctx,
                format!(
                    "__**Queued:**__  `{}` | `{}`",
                    metadata.title.unwrap(),
                    _duration_format(metadata.duration)
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
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
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
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue().current_queue();
        if !queue.is_empty() {
            let mut queue_str = String::new();
            let metadata = queue[0].metadata();
            queue_str += &format!(
                "__**Now playing:**__\n```yaml\n{} | {}\n```",
                shorten(&metadata.title.clone().unwrap(), 40),
                _duration_format(metadata.duration)
            );
            if queue.len() > 1 {
                queue_str += "\n__**Queue:**__\n```yaml\n";
                for (index, track) in queue[1..].iter().take(10).enumerate() {
                    let metadata = track.metadata();
                    queue_str += &format!(
                        "{}: {} | {}\n",
                        index + 1,
                        shorten(&metadata.title.clone().unwrap(), 40),
                        _duration_format(metadata.duration)
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
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
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
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if !queue.is_empty() {
            queue.modify_queue(|q| {
                let mut rng = rand::thread_rng();
                let mut i = q.len();
                while i >= 2 {
                    i -= 1;
                    q.swap(i, rng.gen_range(1, i + 1));
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
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
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
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
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
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
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
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        if let Some(np) = queue.current() {
            let metadata = np.metadata();
            msg.channel_id
                .send_message(ctx, |m| {
                    m.reference_message(msg);
                    _now_playing_embed(m, metadata.as_ref().clone());
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
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
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

/// Play a lofi stream.
///
/// Usage: `lofi <id>`
///
/// Available Channels:
/// ```
/// +-----------------+------------------------------+
/// |       ID        |             URL              |
/// +-----------------+------------------------------+
/// | chilledcow      | https://youtu.be/5qap5aO4i9A |
/// | chilledcow2     | https://youtu.be/DWcJFNfaw9c |
/// | chillhopmusic   | https://youtu.be/5yx6BWlEVcY |
/// | chillhopmusic2  | https://youtu.be/7NOSDKb0HlU |
/// | tokyolosttracks | https://youtu.be/WBfbkPTqUtU |
/// | thejazzhopcafe  | https://youtu.be/OVPPOwMpSpQ |
/// | homeworkradio   | https://youtu.be/ZYMuB9y549s |
/// | steezyasfuck    | https://youtu.be/-5KAN9_CzSA |
/// | thebootlegboy   | https://youtu.be/l7TxwBhtTUY |
/// | inyourchill     | https://youtu.be/B8tQ8RUbTW8 |
/// | collegemusic    | https://youtu.be/bM0Iw7PPoU4 |
/// +-----------------+------------------------------+
/// ```
#[command]
#[num_args(1)]
async fn lofi(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let channel_id = args.message().to_string();
    let url = match channel_id.as_str() {
        "chilledcow" => "https://youtu.be/5qap5aO4i9A",
        "chilledcow2" => "https://youtu.be/DWcJFNfaw9c",
        "chillhopmusic" => "https://youtu.be/5yx6BWlEVcY",
        "chillhopmusic2" => "https://youtu.be/7NOSDKb0HlU",
        "tokyolosttracks" => "https://youtu.be/WBfbkPTqUtU",
        "thejazzhopcafe" => "https://youtu.be/OVPPOwMpSpQ",
        "homeworkradio" => "https://youtu.be/ZYMuB9y549s",
        "steezyasfuck" => "https://youtu.be/-5KAN9_CzSA",
        "thebootlegboy" => "https://youtu.be/l7TxwBhtTUY",
        "inyourchill" => "https://youtu.be/B8tQ8RUbTW8",
        "collegemusic" => "https://youtu.be/bM0Iw7PPoU4",
        _ => {
            msg.reply(
                ctx,
                "Invalid channel ID, try `help lofi` for all the available channels.",
            )
            .await?;
            return Ok(());
        }
    };
    play(ctx, msg, Args::new(url, &[Delimiter::Single(' ')])).await?;

    Ok(())
}

// /// Get lyrics of current song, or search for another.
// ///
// /// Usage: `lyrics <title>`
// #[command]
// #[min_args(1)]
// async fn lyrics(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
//     let title = args.message().to_string();
//     match get_lyrics(title.clone()).await {
//         Ok(lyrics) => {
//             let chars: Vec<char> = lyrics.lyrics.chars().collect();
//             let chunks = chars
//                 .chunks(2000)
//                 .map(|chunk| chunk.iter().collect::<String>())
//                 .collect::<Vec<_>>();
//             let pages = chunks
//                 .iter()
//                 .enumerate()
//                 .map(|(i, chunk)| {
//                     let mut p = CreateMessage::default();
//                     p.embed(|e| {
//                         e.footer(|f| f.text(&format!("Page {}/{}", i + 1, chunks.len())));
//                         e.url(&lyrics.links.genius);
//                         e.thumbnail(&lyrics.thumbnail.genius);
//                         e.title(&format!("{} - {}", lyrics.author, lyrics.title));
//                         e.description(chunk);
//                         e
//                     });
//                     p
//                 })
//                 .collect::<Vec<_>>();
//             let mut menu_options = MenuOptions::default();
//             menu_options.timeout = 180.0;
//             let menu = Menu::new(ctx, msg, &pages, menu_options);
//             menu.run().await?;
//         }
//         Err(_) => {
//             msg.channel_id
//                 .say(
//                     ctx,
//                     &format!("Could not find lyrics for the song: `{}`", title),
//                 )
//                 .await?;
//             return Ok(());
//         }
//     }

//     Ok(())
// }

fn _now_playing_embed(m: &mut CreateMessage, np: Metadata) {
    m.embed(|e| {
        e.title("Now playing");
        e.field("Title", np.title.clone().unwrap(), false);
        if let Some(t) = np.source_url {
            e.field("URL", t, false);
        }
        e.field("Duration", _duration_format(np.duration), true);
        // e.field("Requester", np.requester.mention(), true);
        if let Some(t) = np.thumbnail {
            e.thumbnail(t);
        }
        e
    });
}

fn _duration_format(duration: Option<Duration>) -> String {
    if let Some(d) = duration {
        if d != Duration::default() {
            return format_seconds(d.as_secs());
        }
    }
    "Live".to_string()
}
