use crate::{
    data::{GuildCacheStore, PoolContainer},
    database::Guild,
    service::start_services,
};
use regex::Regex;
use serenity::{
    async_trait,
    model::{
        channel::Message,
        gateway::Ready,
        guild::{Guild as DiscordGuild, Member},
        id::GuildId,
    },
    prelude::{Context, EventHandler},
};
use std::{
    clone::Clone,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tracing::{error, info};

pub struct Handler {
    is_services_running: AtomicBool,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            is_services_running: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Ready) {
        info!("{} is ready!", ready.user.name);
    }

    async fn cache_ready(&self, ctx: Context, _guilds: Vec<GuildId>) {
        let ctx = Arc::new(ctx.clone());
        if *(crate::constants::ENABLE_SERVICES) {
            if !self.is_services_running.load(Ordering::Relaxed) {
                start_services(ctx).await;
                self.is_services_running.swap(true, Ordering::Relaxed);
                info!("Services started");
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        // Ignores bot accounts.
        if msg.author.bot {
            return;
        }

        // Trigger phrase detection and handling.
        if let Some(guild_id) = msg.guild_id {
            let data = ctx.data.read().await;
            if let Some(guild_cache_map) = data.get::<GuildCacheStore>() {
                if let Some(guild_cache) = guild_cache_map.get(guild_id) {
                    for trigger_phrase in &guild_cache.triggers {
                        let re = Regex::new(&format!(r"(\s+|^){}(\s+|$)", &trigger_phrase.phrase))
                            .unwrap();
                        if re.is_match(&msg.content) {
                            if let Err(_) = msg.reply(&ctx, &trigger_phrase.reply).await {
                                error!("Error sending trigger message")
                            }
                        }
                    }
                }
            }
        }

        // Random fun message handling.
        if msg.content.to_lowercase() == "no u" {
            let _ = msg.reply(&ctx, "no u").await;
        }
    }

    async fn guild_create(&self, ctx: Context, guild: DiscordGuild, _flag: bool) {
        let guild_id = guild.id;
        let data = ctx.data.read().await;
        let db = data.get::<PoolContainer>().unwrap();
        let non_bot_members: Vec<i64> = guild
            .members
            .into_iter()
            .filter(|(_id, m)| !m.user.bot)
            .map(|(id, _m)| id.0 as i64)
            .collect();
        let db_guild = Guild::new(db, guild_id);
        if let Err(why) = db_guild.insert().await {
            error!("error adding guild to db {:?}", why);
            return;
        };
        let guild_cache_store = data
            .get::<GuildCacheStore>()
            .expect("Expected GuildCacheStore to be in TypeMap");
        guild_cache_store.update(db, guild_id).await;

        if let Ok(db_members) = db_guild.get_members().await {
            let db_member_ids: Vec<i64> = db_members.iter().map(|m| m.id).collect();
            for id in non_bot_members {
                if !db_member_ids.contains(&id) {
                    if let Err(why) = db_guild.insert_member(id).await {
                        error!("error adding member to db guild {:?}", why);
                        return;
                    };
                }
            }
        }
    }

    async fn guild_member_addition(&self, ctx: Context, guild_id: GuildId, new_member: Member) {
        if !new_member.user.bot {
            let member_id = new_member.user.id;
            let data = ctx.data.read().await;
            let db = data.get::<PoolContainer>().unwrap();
            let db_guild = Guild::new(db, guild_id);
            if let Ok(member) = db_guild.get_member(member_id).await {
                if member.is_none() {
                    if let Err(why) = db_guild.insert_member(member_id).await {
                        error!("error adding member to db guild {:?}", why)
                    }
                }
            }
        }
    }
}
