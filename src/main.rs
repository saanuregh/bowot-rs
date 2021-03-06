mod cache;
mod commands;
mod constants;
mod data;
mod database;
mod framework;
mod handler;
mod service;
mod soundboard;
mod utils;
mod voice;
mod ytdl_cache;

use bb8_redis::{bb8, RedisConnectionManager};
use data::*;
use itconfig::*;
use serenity::{
    client::{bridge::gateway::GatewayIntents, Client},
    http::Http,
};
use songbird::SerenityInit;

use mimalloc::MiMalloc;
use soundboard::init_sound_store;
use sqlx::postgres::PgPoolOptions;
use std::{clone::Clone, collections::HashSet, sync::Arc, time::Duration};
use tokio::{
    signal::unix::{signal, SignalKind},
    time::Instant,
};
use tracing::{error, info};
use tracing_log::env_logger;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let bot_token = get_env::<String>("TOKEN").expect("env::TOKEN not set");
    let http = Http::new_with_token(&bot_token);
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);
            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let std_framework = framework::get_std_framework(owners, bot_id).await;

    let mut client = Client::builder(&bot_token)
        .event_handler(handler::Handler::new())
        .framework(std_framework)
        .intents({
            let mut intents = GatewayIntents::all();
            intents.remove(GatewayIntents::DIRECT_MESSAGE_TYPING);
            intents.remove(GatewayIntents::GUILD_MESSAGE_TYPING);
            intents
        })
        .register_songbird()
        .await?;

    let database_url = get_env::<String>("DATABASE_URL").expect("env::DATABASE_URL not set");
    let db_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await?;

    let redis_url = get_env::<String>("REDIS_URL").expect("env::REDIS_URL not set");
    let redis_manager = RedisConnectionManager::new(redis_url)?;
    let redis_pool = bb8::Pool::builder()
        .connection_timeout(Duration::from_secs(5))
        .build(redis_manager)
        .await?;

    {
        let mut data = client.data.write().await;
        data.insert::<PgPoolContainer>(db_pool.clone());
        data.insert::<RedisPoolContainer>(redis_pool.clone());
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<Uptime>(Instant::now());
        data.insert::<GuildCacheStore>(Arc::new(Default::default()));
        data.insert::<SoundStore>(Arc::new(init_sound_store().await));
    }

    let signal_kinds = vec![
        SignalKind::hangup(),
        SignalKind::interrupt(),
        SignalKind::terminate(),
    ];

    for signal_kind in signal_kinds {
        let mut stream = signal(signal_kind).unwrap();
        let shard_manager = client.shard_manager.clone();
        let db_pool = db_pool.clone();
        let redis_pool = db_pool.clone();

        tokio::spawn(async move {
            stream.recv().await;
            info!("Shutting down");
            shard_manager.lock().await.shutdown_all().await;
            info!("Closing database pool");
            db_pool.close().await;
            info!("Closing redis pool");
            redis_pool.close().await;
            info!("Bye!!!");
        });
    }

    if let Err(why) = client.start_autosharded().await {
        error!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}
