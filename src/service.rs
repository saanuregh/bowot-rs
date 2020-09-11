use crate::{get_all_guilds, MongoClient};
use itconfig::*;
use rand::seq::SliceRandom;
use serenity::{
    model::{gateway::Activity, id::UserId, user::OnlineStatus},
    prelude::Context,
};
use std::collections::HashSet;
use std::{sync::Arc, time::Duration};
use tracing::debug;
use Vec;

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
                                let _ = _u.dm(&ctx.http, |m| m.content("Drink some water")).await;
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    debug!("Hydrate reminder done.");
}

async fn status_update(ctx: Arc<Context>) {
    let status_str: String = get_env_or_default("STATUSES", "uwu,owo,raid shadow legends");
    let statuses: Vec<&str> = status_str.split(",").collect();
    let random_status = statuses.choose(&mut rand::thread_rng()).unwrap();
    ctx.set_presence(Some(Activity::playing(random_status)), OnlineStatus::Online)
        .await;
    debug!("Status update done.");
}

pub async fn service_loop(ctx: Arc<Context>) {
    let ctx_clone1 = Arc::clone(&ctx);
    let ctx_clone2 = Arc::clone(&ctx);
    tokio::spawn(async move {
        loop {
            tokio::join!(hydrate_reminder(Arc::clone(&ctx_clone1)));
            tokio::time::delay_for(Duration::from_secs(2700)).await;
        }
    });
    tokio::spawn(async move {
        loop {
            tokio::join!(status_update(Arc::clone(&ctx_clone2)));
            tokio::time::delay_for(Duration::from_secs(5)).await;
        }
    });
}
