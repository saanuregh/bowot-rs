use crate::utils::basic_functions::format_seconds;
use serenity::{
    async_trait,
    builder::CreateMessage,
    client::Context,
    http::Http,
    model::{channel::Message, id::GuildId, prelude::ChannelId},
    prelude::Mutex,
};
use songbird::{
    input::Metadata, Call, Event, EventContext, EventHandler as VoiceEventHandler, SongbirdKey,
    TrackEvent,
};
use std::{
    sync::atomic::Ordering,
    sync::{atomic::AtomicUsize, Arc},
};

use tokio::time::Duration;
use tracing::error;

pub struct TrackStartNotifier {
    chan_id: ChannelId,
    http: Arc<Http>,
}

#[async_trait]
impl VoiceEventHandler for TrackStartNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track([(_current_track_state, current_track_handle)]) = ctx {
            let metadata = current_track_handle.metadata();
            if let Some(source_url) = &metadata.source_url {
                if source_url.starts_with("soundboard_") {
                    return None;
                }
            }
            if let Err(why) = self
                .chan_id
                .send_message(&self.http, |m| {
                    get_now_playing_embed(m, metadata.clone());
                    m
                })
                .await
            {
                error!("Error sending message: {:?}", why);
            }
        }

        None
    }
}

pub struct ChannelIdleChecker {
    ctx: Arc<Context>,
    guild_id: GuildId,
    elapsed: AtomicUsize,
}

#[async_trait]
impl VoiceEventHandler for ChannelIdleChecker {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let data = self.ctx.data.read().await;
        if let Some(manager) = data.get::<SongbirdKey>() {
            if let Some(handler_lock) = manager.get(self.guild_id) {
                let queue_empty = {
                    let handler = handler_lock.lock().await;
                    handler.queue().is_empty()
                };
                if queue_empty {
                    if (self.elapsed.fetch_add(1, Ordering::Relaxed) + 1) > 2 {
                        let _ = manager.remove(self.guild_id).await;

                        return Some(Event::Cancel);
                    }
                } else {
                    self.elapsed.store(0, Ordering::Relaxed);
                }

                return None;
            }
        }
        error!("Something error happened in channel idle checking, canceling handler");

        Some(Event::Cancel)
    }
}

pub async fn join_voice_channel(ctx: &Context, msg: &Message) -> Option<Arc<Mutex<Call>>> {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    if let Some(connect_to) = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
    {
        let data = ctx.data.read().await;
        let manager = data
            .get::<SongbirdKey>()
            .expect("Expected Songbird in TypeMap");
        let (handler_lock, success) = manager.join(guild.id, connect_to).await;
        if let Err(why) = success {
            error!("Error while joining voice channel: {:?}", why);

            return None;
        }
        {
            let mut handler = handler_lock.lock().await;
            handler.add_global_event(
                Event::Track(TrackEvent::Play),
                TrackStartNotifier {
                    chan_id: msg.channel_id,
                    http: ctx.http.clone(),
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

pub fn get_now_playing_embed(m: &mut CreateMessage, np: Metadata) {
    m.embed(|e| {
        e.title("Now playing");
        e.field("Title", np.title.clone().unwrap(), false);
        if let Some(t) = np.source_url {
            e.field("URL", t, false);
        }
        e.field("Duration", format_duration(np.duration), true);
        // e.field("Requester", np.requester.mention(), true);
        if let Some(t) = np.thumbnail {
            e.thumbnail(t);
        }
        e
    });
}

pub fn format_duration(duration: Option<Duration>) -> String {
    if let Some(d) = duration {
        if d != Duration::default() {
            return format_seconds(d.as_secs());
        }
    }
    "Live".to_string()
}
