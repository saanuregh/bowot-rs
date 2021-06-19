use serenity::{
    client::bridge::gateway::ShardManager,
    prelude::{Mutex, RwLock, TypeMapKey},
};
use songbird::input::cached::Compressed;
use sqlx::PgPool;
use std::{collections::HashMap, sync::Arc};
use tokio::time::Instant;

pub struct ShardManagerContainer;
impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub struct PoolContainer;

impl TypeMapKey for PoolContainer {
    type Value = PgPool;
}

pub struct Uptime;

impl TypeMapKey for Uptime {
    type Value = Instant;
}
pub struct PrefixCache;

impl TypeMapKey for PrefixCache {
    type Value = Arc<RwLock<HashMap<i64, String>>>;
}

pub struct SoundStore;

impl TypeMapKey for SoundStore {
    type Value = Arc<Mutex<HashMap<String, Compressed>>>;
}
