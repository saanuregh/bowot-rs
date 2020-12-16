use crate::{
    database::get_all_guilds,
    lang::{HYDRATE, STATUSES},
    Database,
};
use rand::seq::SliceRandom;
use serenity::{
    model::{gateway::Activity, id::UserId, user::OnlineStatus},
    prelude::Context,
};
use std::collections::HashSet;
use std::{sync::Arc, time::Duration};
use tracing::{error, info};

async fn hydrate_reminder(ctx: Arc<Context>) {
    let client = ctx.http.clone();
    let data = ctx.data.read().await;
    let mongo_client = data.get::<Database>().unwrap();
    let mut users: HashSet<i64> = HashSet::new();
    if let Ok(guilds) = get_all_guilds(mongo_client).await {
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
                                }
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    info!(
        "Hydrate reminder done, successfully send reminders to {} users",
        users.len()
    );
}

async fn status_update(ctx: Arc<Context>) {
    let random_status = STATUSES.choose(&mut rand::thread_rng()).unwrap();
    let activity = match random_status[0] {
        "playing" => Activity::playing,
        "competing" => Activity::competing,
        "listening" => Activity::listening,
        _ => Activity::playing,
    };
    ctx.set_presence(Some(activity(random_status[1])), OnlineStatus::Online)
        .await;
    info!("Status update done");
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
            tokio::time::delay_for(Duration::from_secs(1800)).await;
        }
    });
}
