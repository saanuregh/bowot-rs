use crate::{get_all_guilds, MongoClient, PlayerManager, VoiceManager};
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use serenity::{
    model::{gateway::Activity, id::UserId, user::OnlineStatus},
    prelude::Context,
};
use std::collections::HashSet;
use std::{sync::Arc, time::Duration};
use tracing::{debug, error, info};

lazy_static! {
    static ref STATUSES: Vec<&'static str> =
        include_str!("data/statuses.txt").split('\n').collect();
    static ref HYDRATE: Vec<&'static str> = include_str!("data/hydrate.txt").split('\n').collect();
}

async fn hydrate_reminder(ctx: Arc<Context>) {
    let client = ctx.http.clone();
    let data = ctx.data.read().await;
    let mongo_client = data.get::<MongoClient>().unwrap();
    if let Ok(guilds) = get_all_guilds(mongo_client).await {
        let mut users: HashSet<i64> = HashSet::new();
        for g in guilds {
            for u in g.hydrate {
                users.insert(u);
            }
        }
        for u in users.iter() {
            let user_id = *u as u64;
            for guild_id in &ctx.cache.guilds().await {
                if let Some(_g) = &ctx.cache.guild(guild_id).await {
                    if let Some(x) = _g.presences.get(&UserId(user_id)) {
                        if x.status.name() == OnlineStatus::Online.name() {
                            if let Ok(_u) = client.get_user(user_id).await {
                                let random_msg = HYDRATE.choose(&mut rand::thread_rng()).unwrap();
                                if let Err(error) =
                                    _u.dm(&ctx.http, |m| m.content(random_msg)).await
                                {
                                    error!("Unhandled dispatch error: {:?}", error);
                                } else {
                                    info!("Successfully send to user: {:?}", user_id);
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    debug!("Hydrate reminder done");
}

async fn status_update(ctx: Arc<Context>) {
    let random_status = STATUSES.choose(&mut rand::thread_rng()).unwrap();
    ctx.set_presence(Some(Activity::playing(random_status)), OnlineStatus::Online)
        .await;
    debug!("Status update done");
}

async fn check_vc_empty(ctx: Arc<Context>) {
    let data = ctx.data.read().await;
    let manager_lock = data
        .get::<VoiceManager>()
        .cloned()
        .expect("Expected VoiceManager in ShareMap.");
    let player_lock = data
        .get::<PlayerManager>()
        .cloned()
        .expect("Expected Player Manger in TypeMap");
    for guild_id in &ctx.cache.guilds().await {
        let mut manager = manager_lock.lock().await;
        if let Some(handler) = manager.get_mut(guild_id) {
            let guild = ctx.cache.guild(guild_id).await.unwrap();
            if let Some(channel) = guild.channels.get(&handler.channel_id.unwrap()) {
                if let Ok(members) = channel.members(&ctx).await {
                    if members.len() == 1 {
                        let mut pm = player_lock.write().await;
                        if let Some(player) = pm.get_mut(&(guild_id.0 as u64)) {
                            if !player.is_finished().await {
                                player.reset();
                                handler.stop();
                            }
                            pm.remove(&(guild_id.0 as u64));
                        }
                        manager.remove(guild_id);
                        info!("Removed VC cause of inactivity in guild: {:?}", guild_id);
                    }
                }
            }
        }
    }
    debug!("VC empty check done");
}

pub async fn service_loop(ctx: Arc<Context>) {
    let ctx_clone1 = Arc::clone(&ctx);
    let ctx_clone2 = Arc::clone(&ctx);
    let ctx_clone3 = Arc::clone(&ctx);
    tokio::spawn(async move {
        loop {
            tokio::join!(hydrate_reminder(Arc::clone(&ctx_clone1)));
            tokio::time::delay_for(Duration::from_secs(2700)).await;
        }
    });
    tokio::spawn(async move {
        loop {
            tokio::join!(status_update(Arc::clone(&ctx_clone2)));
            tokio::time::delay_for(Duration::from_secs(1800)).await;
        }
    });
    tokio::spawn(async move {
        loop {
            tokio::join!(check_vc_empty(Arc::clone(&ctx_clone3)));
            tokio::time::delay_for(Duration::from_secs(900)).await;
        }
    });
}
