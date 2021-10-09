use std::{sync::Arc, time::Duration};

use lavalink_rs::gateway::LavalinkEventHandler;
use poise::{
    serenity::async_trait,
    serenity_prelude::{Channel, GuildId, Http, Mentionable, UserId},
};
use songbird::Songbird;
use tracing::{debug, info};

use crate::{
    constants::MAX_SINGLE_ENTRY_LENGTH,
    types::{IdleHashMap, LastMessageHashMap},
    utils::helpers::{chop_str, display_time_span},
};

pub struct LavalinkHandler {
    guild_last_message_map: LastMessageHashMap,
    guild_idle_map: IdleHashMap,
    http: Arc<Http>,
    songbird: Arc<Songbird>,
}

impl LavalinkHandler {
    pub fn new(
        guild_last_message_map: LastMessageHashMap,
        guild_idle_map: IdleHashMap,
        http: Arc<Http>,
        songbird: Arc<Songbird>,
    ) -> Self {
        Self {
            guild_last_message_map,
            guild_idle_map,
            http,
            songbird,
        }
    }
}

const MAX_IDLE: Duration = Duration::from_secs(900);

#[async_trait]
impl LavalinkEventHandler for LavalinkHandler {
    async fn stats(
        &self,
        lava_client: lavalink_rs::LavalinkClient,
        event: lavalink_rs::model::Stats,
    ) {
        let guild_idle_map = self.guild_idle_map.read().await;
        for (guild_id, instant) in guild_idle_map.iter() {
            let elapsed = instant.elapsed();
            if elapsed > MAX_IDLE {
                let _ = self.songbird.remove(*guild_id).await;
                let _ = lava_client.destroy(*guild_id).await;
                {
                    let mut guild_idle_map = self.guild_idle_map.write().await;
                    guild_idle_map.remove(guild_id);
                }
            }
        }
        info!("{:?}", event)
    }

    async fn player_update(
        &self,
        _client: lavalink_rs::LavalinkClient,
        event: lavalink_rs::model::PlayerUpdate,
    ) {
        debug!("{:?}", event)
    }

    async fn track_start(
        &self,
        _client: lavalink_rs::LavalinkClient,
        event: lavalink_rs::model::TrackStart,
    ) {
        {
            let mut idle_time_map = self.guild_idle_map.write().await;
            idle_time_map.insert(event.guild_id.0, tokio::time::Instant::now());
        }
        info!("{:?}", event)
    }

    async fn track_finish(
        &self,
        client: lavalink_rs::LavalinkClient,
        event: lavalink_rs::model::TrackFinish,
    ) {
        let guild_id = GuildId(event.guild_id.0);
        if let Some(node) = client.nodes().await.get(&guild_id.0) {
            if let Some(next_track) = node.queue.first() {
                let last_message_map = self.guild_last_message_map.read().await;
                if let Some(channel_id) = last_message_map.get(&guild_id.0) {
                    if let Ok(channel) = self.http.get_channel(channel_id.0).await {
                        match channel {
                            Channel::Guild(guild_channel) => {
                                let track_info = next_track.track.info.as_ref().unwrap();

                                let _ = guild_channel
                                    .send_message(&self.http, |m| {
                                        m.embed(|e| {
                                            e.title("Now Playing")
                                                .field(
                                                    "Track:",
                                                    format!(
                                                        "[{}]({})",
                                                        chop_str(
                                                            track_info.title.as_str(),
                                                            MAX_SINGLE_ENTRY_LENGTH
                                                        ),
                                                        track_info.uri,
                                                    ),
                                                    false,
                                                )
                                                .field(
                                                    "Duration:",
                                                    display_time_span(track_info.length),
                                                    true,
                                                )
                                                .field(
                                                    "Requested By:",
                                                    UserId(
                                                        next_track
                                                            .requester
                                                            .expect(
                                                                "Expected a requester associated \
																 with a playing track",
                                                            )
                                                            .0,
                                                    )
                                                    .mention(),
                                                    true,
                                                )
                                        })
                                    })
                                    .await;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    async fn websocket_closed(
        &self,
        _client: lavalink_rs::LavalinkClient,
        event: lavalink_rs::model::WebSocketClosed,
    ) {
        info!("{:?}", event)
    }

    async fn player_destroyed(
        &self,
        _client: lavalink_rs::LavalinkClient,
        event: lavalink_rs::model::PlayerDestroyed,
    ) {
        info!("{:?}", event)
    }

    async fn track_exception(
        &self,
        _client: lavalink_rs::LavalinkClient,
        event: lavalink_rs::model::TrackException,
    ) {
        info!("{:?}", event)
    }
}
