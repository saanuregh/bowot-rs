use crate::{
    database::{get_all_cmd_stats, get_all_guilds},
    lang::{HYDRATE, STATUSES},
    utils::basic_functions::{
        get_meta_info, get_process_usage, get_shard_latency, get_uptime, merge_json,
    },
    Database,
};
use handlebars::Handlebars;
use lazy_static::lazy_static;
use rand::seq::SliceRandom;
use serde_json::json;
use serenity::{
    model::{gateway::Activity, id::UserId, user::OnlineStatus},
    prelude::Context,
};
use std::{collections::HashSet, convert::Infallible};
use std::{sync::Arc, time::Duration};
use tracing::{error, info};
use warp::{sse::ServerSentEvent, Filter};

lazy_static! {
    static ref HB: Handlebars<'static> = {
        let mut hb = Handlebars::new();
        hb.register_template_string("index.html", include_str!("templates/index.hbs"))
            .unwrap();
        hb
    };
}

fn with_context(
    ctx: Arc<Context>,
) -> impl Filter<Extract = (Arc<Context>,), Error = Infallible> + Clone {
    warp::any().map(move || ctx.clone())
}

async fn render_index(ctx: Arc<Context>) -> Result<Box<dyn warp::Reply>, Infallible> {
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    let mut total_cmd_stat: i64 = 0;
    let mut popular_cmd = String::new();
    let mut popular_cmd_count: i64 = 0;
    if let Ok(all_cmd_stats) = get_all_cmd_stats(db).await {
        for cmd_stat in all_cmd_stats {
            if cmd_stat.count > popular_cmd_count {
                popular_cmd_count = cmd_stat.count;
                popular_cmd = cmd_stat.cmd_name;
            }
            total_cmd_stat += cmd_stat.count;
        }
    }
    let mut meta = json!(get_meta_info(ctx.as_ref()).await);
    let b = json!({
        "popular_cmd": popular_cmd,
        "total_cmd_stat": total_cmd_stat,
    });
    merge_json(&mut meta, &b);
    let render = HB
        .render("index.html", &meta)
        .unwrap_or_else(|err| err.to_string());
    Ok(Box::new(warp::reply::html(render)))
}

async fn send_stats(ctx: Arc<Context>) -> Result<Box<dyn warp::Reply>, Infallible> {
    Ok(Box::new(warp::sse::reply(stats_stream_sse(ctx).await)))
}

async fn stats_stream_sse(
    ctx: Arc<Context>,
) -> impl futures::Stream<Item = Result<impl ServerSentEvent, Infallible>> {
    let (cpu_usage, memory_usage) = get_process_usage();
    let uptime = get_uptime(&ctx).await;
    let shard_latency = get_shard_latency(&ctx).await;
    futures::stream::iter(vec![Ok(warp::sse::data(
        json!({
            "cpu_usage": cpu_usage,
            "memory_usage": memory_usage,
            "uptime": uptime,
            "shard_latency": shard_latency,
        })
        .to_string(),
    ))])
}

pub async fn routes(
    ctx: Arc<Context>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let index = warp::get()
        .and(warp::path::end())
        .and(with_context(ctx.clone()))
        .and_then(render_index);
    let stats = warp::path("stats")
        .and(warp::get())
        .and(with_context(ctx.clone()))
        .and_then(send_stats);
    index.or(stats)
}

async fn hydrate_reminder(ctx: Arc<Context>) {
    let client = ctx.http.clone();
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    let mut users: HashSet<i64> = HashSet::new();
    if let Ok(guilds) = get_all_guilds(db).await {
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

pub async fn start_services(ctx: Arc<Context>) {
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
        warp::serve(routes(Arc::clone(&ctx_clone3)).await)
            .run(([127, 0, 0, 1], 3000))
            .await;
    });
}
