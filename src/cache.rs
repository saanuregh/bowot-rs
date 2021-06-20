use dashmap::{mapref::one::Ref, DashMap};
use sqlx::PgPool;
use tracing::info;

use crate::database::{Guild, Trigger};

pub struct GuildCache {
    pub guild_id: i64,
    pub prefix: String,
    pub disabled_commands: Vec<String>,
    pub triggers: Vec<Trigger>,
}

impl GuildCache {
    async fn new(pool: &PgPool, guild_id: impl Into<i64>) -> Self {
        let guild_id: i64 = guild_id.into();
        let db_guild = Guild::new(pool, guild_id);
        let prefix = db_guild.get_prefix().await.unwrap();
        let disabled_commands = db_guild.get_disabled_commands().await.unwrap();
        let triggers = db_guild.get_triggers().await.unwrap();
        Self {
            guild_id,
            prefix,
            disabled_commands,
            triggers,
        }
    }

    async fn update_cache(&mut self, pool: &PgPool) {
        let updated = GuildCache::new(pool, self.guild_id).await;
        self.prefix = updated.prefix;
        self.disabled_commands = updated.disabled_commands;
        self.triggers = updated.triggers;
    }
}

pub struct GuildCacheMap(DashMap<i64, GuildCache>);

impl GuildCacheMap {
    pub fn new() -> Self {
        Self { 0: DashMap::new() }
    }

    pub async fn insert(&self, pool: &PgPool, guild_id: impl Into<i64>) {
        let guild_id: i64 = guild_id.into();
        self.0
            .insert(guild_id, GuildCache::new(pool, guild_id).await);
    }

    pub fn get(&self, guild_id: impl Into<i64>) -> Option<Ref<'_, i64, GuildCache>> {
        let guild_id: i64 = guild_id.into();
        self.0.get(&guild_id)
    }

    pub async fn update(&self, pool: &PgPool, guild_id: impl Into<i64>) {
        let guild_id: i64 = guild_id.into();
        if let Some(guild_cache) = self.0.get_mut(&guild_id).as_deref_mut() {
            guild_cache.update_cache(pool).await;
        }
        self.insert(pool, guild_id).await;
        info!("Cache for guild {} updated", guild_id);
    }
}

impl Default for GuildCacheMap {
    fn default() -> Self {
        Self::new()
    }
}
