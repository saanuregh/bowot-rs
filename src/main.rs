mod commands;
mod constants;
mod data;
mod database;
mod framework;
mod lavalink;
mod services;
mod types;
mod utils;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Context;
use dotenv;
use framework::get_framework_builder;
use itconfig::*;
use lavalink_rs::LavalinkClient;
use poise::{
    serenity::{client::parse_token, http::Http},
    serenity_prelude::{GatewayIntents, RwLock},
};
use songbird::{SerenityInit, Songbird};
use sqlx::postgres::PgPoolOptions;
use tokio::time::Instant;
use tracing::info;
use tracing_log::env_logger;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use crate::{
    data::{Data, IdleGuildMap, LastMessageMap, PgPoolContainer, Uptime},
    lavalink::LavalinkHandler,
    types::{IdleHashMap, LastMessageHashMap},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;
    env_logger::init();
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting bot");

    let bot_token = get_env::<String>("TOKEN")?;
    let bot_id = parse_token(bot_token.clone())
        .with_context(|| "Token is invalid".to_owned())?
        .bot_user_id;

    let http = Http::new_with_token(&bot_token);
    let owner_id = http
        .get_current_application_info()
        .await
        .with_context(|| "Failed to get application info".to_owned())?
        .owner
        .id;

    let mut owners = HashSet::new();
    owners.insert(owner_id);

    let database_url = get_env::<String>("DATABASE_URL")?;
    let db_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&database_url)
        .await
        .with_context(|| "Failed to connect to database".to_owned())?;
    let db_pool_clone = db_pool.clone();

    info!("Database client started");

    let last_message_map: LastMessageHashMap = Arc::new(RwLock::new(HashMap::new()));
    let last_message_map_clone = last_message_map.clone();

    let idle_hash_map: IdleHashMap = Arc::new(RwLock::new(HashMap::new()));
    let idle_hash_map_clone = idle_hash_map.clone();

    info!("Lavalink client started");

    let songbird = Songbird::serenity();
    let songbird_clone = songbird.clone();

    get_framework_builder(bot_token, owners)
        .user_data_setup(move |ctx, _ready, _framework| {
            Box::pin(async move {
                let lavalink_host = get_env::<String>("LAVALINK_HOST")?;
                let lavalink_password = get_env::<String>("LAVALINK_PASSWORD")?;
                let lavalink = LavalinkClient::builder(bot_id.0)
                    .set_host(lavalink_host)
                    .set_password(lavalink_password)
                    .build(LavalinkHandler::new(
                        last_message_map_clone,
                        idle_hash_map_clone,
                        ctx.http.clone(),
                        songbird_clone.clone(),
                    ))
                    .await
                    .with_context(|| "Failed to start the Lavalink client")?;
                Ok(Data::new(songbird_clone, lavalink))
            })
        })
        .client_settings(|client_builder| {
            client_builder
                .intents({
                    let mut intents = GatewayIntents::all();
                    intents.remove(GatewayIntents::DIRECT_MESSAGE_TYPING);
                    intents.remove(GatewayIntents::GUILD_MESSAGE_TYPING);
                    intents
                })
                .register_songbird_with(songbird)
                .type_map_insert::<PgPoolContainer>(db_pool_clone)
                .type_map_insert::<Uptime>(Instant::now())
                .type_map_insert::<LastMessageMap>(last_message_map)
                .type_map_insert::<IdleGuildMap>(idle_hash_map)
        })
        .run()
        .await
        .with_context(|| "Failed to start the bot".to_owned())?;

    Ok(())
}
