use crate::cache::GuildCacheMap;
use bb8_redis::{bb8::Pool, RedisConnectionManager};
use dashmap::DashMap;
use serenity::{
    client::bridge::gateway::ShardManager,
    prelude::{Mutex, TypeMapKey},
};
use songbird::input::cached::Compressed;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::time::Instant;

pub struct ShardManagerContainer;
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub struct PgPoolContainer;

impl TypeMapKey for PgPoolContainer {
    type Value = PgPool;
}

pub struct Uptime;

impl TypeMapKey for Uptime {
    type Value = Instant;
}
pub struct GuildCacheStore;

impl TypeMapKey for GuildCacheStore {
    type Value = Arc<GuildCacheMap>;
}

pub struct SoundStore;

impl TypeMapKey for SoundStore {
    type Value = Arc<DashMap<String, Compressed>>;
}

pub struct RedisPoolContainer;

impl TypeMapKey for RedisPoolContainer {
    type Value = Pool<RedisConnectionManager>;
}
