use crate::{
    constants::{HYDRATE, PORT, STATUSES},
    data::PoolContainer,
    database::HydrateReminder,
};
use rand::seq::SliceRandom;
use serenity::{
    model::{gateway::Activity, id::UserId, user::OnlineStatus},
    prelude::Context,
};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{error, info};
use warp::Filter;

async fn routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let health = warp::path("health").map(|| "OK");
    let index = warp::get().map(|| "OK");
    health.or(index)
}

async fn hydrate_reminder(ctx: Arc<Context>) {
    let client = ctx.http.clone();
    let data = ctx.data.read().await;
    let db = data.get::<PoolContainer>().unwrap();
    if let Ok(members) = HydrateReminder::new(db).get_all().await {
        for u in members.iter() {
            let user_id = *u as u64;
            for guild_id in &ctx.cache.guilds().await {
                if let Some(guild) = &ctx.cache.guild(guild_id).await {
                    if let Some(x) = guild.presences.get(&UserId(user_id)) {
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
        info!(
            "Hydrate reminder done, successfully send reminders to {} users",
            members.len()
        );
    }
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
pub async fn start_services(ctx: Arc<Context>) {
    let ctx_clone1 = Arc::clone(&ctx);
    let ctx_clone2 = Arc::clone(&ctx);
    tokio::spawn(async move {
        loop {
            tokio::join!(status_update(Arc::clone(&ctx_clone1)));
            tokio::time::sleep(Duration::from_secs(1800)).await;
        }
    });
    tokio::spawn(async move {
        loop {
            tokio::join!(hydrate_reminder(Arc::clone(&ctx_clone2)));
            tokio::time::sleep(Duration::from_secs(2700)).await;
        }
    });
    tokio::spawn(async move {
        warp::serve(routes().await).run(([0, 0, 0, 0], *PORT)).await;
    });
}
