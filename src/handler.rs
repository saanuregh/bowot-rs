use crate::{database::Guild, service::service_loop, Config, MongoClient};
use regex::Regex;
use reqwest::Client as ReqwestClient;
use serenity::{
    async_trait,
    model::{
        channel::Message,
        gateway::Ready,
        guild::{Guild as DiscordGuild, Member as DiscordMember, PartialGuild, Role},
        id::{GuildId, RoleId},
        user::User,
    },
    prelude::{Context, EventHandler},
};
use std::{clone::Clone, collections::HashMap, sync::Arc};
use tracing::{error, info};

pub struct Handler; // Defines the handler to be used for events.

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        let ctx = Arc::new(ctx);
        if (ctx.data.read().await.get::<Config>().unwrap()["bot"]["enable_service"]
            .as_bool()
            .unwrap_or(false))
        {
            info!("Starting services");
            tokio::join!(service_loop(Arc::clone(&ctx)));
        }
        info!("{} is ready!", ready.user.name);
    }

    async fn message(&self, ctx: Context, msg: Message) {
        // Ignores bot accounts.
        if msg.author.bot {
            return;
        }

        // Trigger phrase detection and handling.
        let guild_id = msg.guild_id.unwrap().0 as i64;
        let data_read = ctx.data.read().await;
        let client = data_read.get::<MongoClient>().unwrap();
        if let Ok(guild) = Guild::from_db(client, guild_id).await {
            for trigger_phrase in guild.trigger_phrases {
                let re = Regex::new(&format!(r"(\s+|^){}(\s+|$)", &trigger_phrase.phrase)).unwrap();
                if re.is_match(&msg.content) {
                    if let Ok(_) = msg.channel_id.say(&ctx, &trigger_phrase.reply).await {
                        if !trigger_phrase.emote.is_whitespace() {
                            let _ = msg.react(&ctx, trigger_phrase.emote).await;
                        }
                    }
                }
            }
        }

        // Random fun message handling.
        if msg.content.to_lowercase() == "no u" {
            let _ = msg.reply(&ctx, "no u").await;
        }

        // Conversational AI when mentioned.
        if msg.mentions_user_id(ctx.cache.current_user().await.id) {
            let input = Regex::new("<@[0-9]+>")
                .unwrap()
                .replace_all(&msg.content, "")
                .to_string();
            let mut map = HashMap::new();
            map.insert("instance", "18281268");
            map.insert("application", "2615581203305355564");
            map.insert("message", &input);
            let client = ReqwestClient::new();
            if let Ok(resp) = client
                .post("https://www.botlibre.com/rest/json/chat")
                .json(&map)
                .send()
                .await
            {
                if let Ok(data) = resp.json::<HashMap<String, String>>().await {
                    if !data["message"].is_empty() {
                        let _ = msg.reply(&ctx, &data["message"]).await;
                    }
                }
            }
        }
    }

    async fn guild_create(&self, ctx: Context, guild: DiscordGuild, _flag: bool) {
        let guild_id = guild.id.0 as i64;
        let data_read = ctx.data.read().await;
        let client = data_read.get::<MongoClient>().unwrap();
        if let Err(_) = Guild::from_db(client, guild_id).await {
            let p = data_read.get::<Config>().unwrap()["bot"]["prefix"]
                .as_str()
                .unwrap()
                .to_string();
            let mut _guild = Guild::new(guild_id, p);
            guild.members.iter().for_each(|(id, m)| {
                if !m.user.bot {
                    if let Err(e) = _guild.add_member(id.0 as i64) {
                        error!("{:?}", e);
                    }
                }
            });
            if let Err(e) = _guild.save_guild(client).await {
                error!("{:?}", e);
            }
        }
    }

    async fn guild_delete(&self, ctx: Context, guild: PartialGuild, _full: Option<DiscordGuild>) {
        let guild_id = guild.id.0 as i64;
        let data_read = ctx.data.read().await;
        let client = data_read.get::<MongoClient>().unwrap();
        if let Ok(mut g) = Guild::from_db(client, guild_id).await {
            if let Err(e) = g.delete_guild(client).await {
                error!("{:?}", e);
            }
        }
    }

    async fn guild_member_addition(
        &self,
        ctx: Context,
        guild_id: GuildId,
        mut new_member: DiscordMember,
    ) {
        if new_member.user.bot {
            return;
        }
        let data_read = ctx.data.read().await;
        let client = data_read.get::<MongoClient>().unwrap();
        if let Ok(mut g) = Guild::from_db(client, guild_id.0 as i64).await {
            if let Ok(g) = g.add_member(new_member.user.id.0 as i64) {
                match g.save_guild(client).await {
                    Ok(g) => {
                        if g.default_role != 0 {
                            let _ = new_member
                                .add_role(ctx.clone(), g.default_role as u64)
                                .await;
                        }
                    }
                    Err(e) => error!("{:?}", e),
                }
            }
        }
    }

    async fn guild_member_removal(
        &self,
        ctx: Context,
        guild_id: GuildId,
        user: User,
        _member_data_if_available: Option<DiscordMember>,
    ) {
        if user.bot {
            return;
        }
        let data_read = ctx.data.read().await;
        let client = data_read.get::<MongoClient>().unwrap();
        if let Ok(mut _g) = Guild::from_db(client, guild_id.0 as i64).await {
            if let Ok(g) = _g.remove_member(user.id.0 as i64) {
                if let Err(e) = g.save_guild(client).await {
                    error!("{:?}", e);
                }
            }
        }
    }

    async fn guild_role_delete(
        &self,
        ctx: Context,
        guild_id: GuildId,
        removed_role_id: RoleId,
        _removed_role_data_if_available: Option<Role>,
    ) {
        let data_read = ctx.data.read().await;
        let client = data_read.get::<MongoClient>().unwrap();
        let mut change = false;
        let role_id = removed_role_id.0 as i64;
        if let Ok(mut guild) = Guild::from_db(client, guild_id.0 as i64).await {
            if guild.default_role == role_id {
                if let Err(e) = guild.change_default_role(0) {
                    error!("{:?}", e);
                    return;
                }
                change = true;
            }
            if let Ok(_) = guild.self_roles.binary_search(&role_id) {
                if let Err(e) = guild.remove_self_role(role_id) {
                    error!("{:?}", e);
                    return;
                }
                change = true;
            }
            if change {
                if let Err(e) = guild.save_guild(client).await {
                    error!("{:?}", e);
                }
            }
        }
    }
}
