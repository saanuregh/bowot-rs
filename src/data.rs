use std::sync::{atomic::AtomicBool, Arc};

use lavalink_rs::LavalinkClient;
use poise::serenity_prelude::TypeMapKey;
use songbird::Songbird;
use sqlx::PgPool;
use tokio::time::Instant;

use crate::types::{IdleHashMap, LastMessageHashMap};

pub struct PgPoolContainer;

impl TypeMapKey for PgPoolContainer {
    type Value = PgPool;
}

pub struct Uptime;

impl TypeMapKey for Uptime {
    type Value = Instant;
}

pub struct LastMessageMap;

impl TypeMapKey for LastMessageMap {
    type Value = LastMessageHashMap;
}

pub struct IdleGuildMap;

impl TypeMapKey for IdleGuildMap {
    type Value = IdleHashMap;
}

pub struct Data {
    pub songbird: Arc<Songbird>,
    pub lavalink: LavalinkClient,
    pub is_services_running: AtomicBool,
}

impl Data {
    pub fn new(songbird: Arc<Songbird>, lavalink: LavalinkClient) -> Self {
        Self {
            songbird,
            lavalink,
            is_services_running: Default::default(),
        }
    }
}
