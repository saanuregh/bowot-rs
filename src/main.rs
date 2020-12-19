mod commands;
mod constants;
mod database;
mod framework;
mod handler;
mod service;
mod utils;

use itconfig::*;
use serenity::{
    client::{
        bridge::gateway::{GatewayIntents, ShardManager},
        Client,
    },
    http::Http,
    prelude::TypeMapKey,
};
use songbird::SerenityInit;
use std::{
    clone::Clone,
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, instrument, Level};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;
use wither::mongodb::{Client as MongoClient, Database as MongoDatabase};

struct ShardManagerContainer; // Shard manager to use for the latency.
struct Database; // The connection to the mongo database.
struct Uptime; //  This is for the startup time of the bot.
struct PrefixCache; //  This is for caching prefix.

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

impl TypeMapKey for Database {
    type Value = MongoDatabase;
}

impl TypeMapKey for Uptime {
    type Value = Instant;
}

impl TypeMapKey for PrefixCache {
    type Value = Arc<RwLock<HashMap<i64, String>>>;
}

#[tokio::main]
#[instrument]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if *(constants::TRACING) {
        LogTracer::init()?;
        let base_level = *(constants::TRACE_LEVEL);
        let level = match base_level {
            "error" => Level::ERROR,
            "warn" => Level::WARN,
            "info" => Level::INFO,
            "debug" => Level::DEBUG,
            "trace" => Level::TRACE,
            _ => Level::INFO,
        };
        info!("Tracer initialized");
        let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
        tracing::subscriber::set_global_default(subscriber)?;
        info!("Subscriber initialized");
    }
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
        .event_handler(handler::Handler)
        .framework(std_framework)
        .intents({
            let mut intents = GatewayIntents::all();
            intents.remove(GatewayIntents::DIRECT_MESSAGE_TYPING);
            intents.remove(GatewayIntents::GUILD_MESSAGE_TYPING);
            intents
        })
        .register_songbird()
        .await?;

    // Block to define global data.
    // and so the data lock is not kept open in write mode.
    {
        // Open the data lock in write mode.
        let mut data = client.data.write().await;

        // Add the database connection to the data.
        {
            let mongo_uri = get_env::<String>("DATABASE_URL").expect("env::DATABASE_URL not set");
            let mongo_database = MongoClient::with_uri_str(&mongo_uri)
                .await?
                .database(*(constants::DATABASE));
            data.insert::<Database>(mongo_database.clone());
            info!("Database initialized");
        }

        // Add the shard manager to the data.
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));

        // Set current time as the uptime.
        data.insert::<Uptime>(Instant::now());

        data.insert::<PrefixCache>(Arc::new(RwLock::new(HashMap::new())));
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start_autosharded().await {
        error!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}
