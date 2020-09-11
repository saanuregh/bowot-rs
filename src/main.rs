mod commands;
mod database;
mod framework;
mod handler;
mod player;
mod service;
mod utils;

use database::*;
use framework::*;
use handler::Handler;

use std::{
    clone::Clone,
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};
use tokio::sync::Mutex;
use tracing::{error, info, instrument, Level};
use tracing_log::LogTracer;
use tracing_subscriber::FmtSubscriber;

use itconfig::*;
use serenity::{
    client::{
        bridge::{gateway::ShardManager, voice::ClientVoiceManager},
        Client,
    },
    http::Http,
    prelude::{RwLock, TypeMapKey},
};
use wither::mongodb::Client as Mongo;

struct ShardManagerContainer; // Shard manager to use for the latency.
struct MongoClient; // The connection to the mongo database.
struct Uptime; //  This is for the startup time of the bot.
struct VoiceManager; //  This is the struct for the voice manager.
struct PlayerManager; //  This is the struct for the player manager.

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

impl TypeMapKey for MongoClient {
    type Value = Mongo;
}

impl TypeMapKey for Uptime {
    type Value = Instant;
}

impl TypeMapKey for VoiceManager {
    type Value = Arc<Mutex<ClientVoiceManager>>;
}

impl TypeMapKey for PlayerManager {
    type Value = Arc<RwLock<HashMap<u64, player::Player>>>;
}

#[tokio::main(core_threads = 8)]
#[instrument]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if get_env_or_default::<bool, bool>("TRACING", false) {
        LogTracer::init()?;
        let base_level: &str = get_env_or_default("TRACE_LEVEL", "info");
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

    let std_framework = get_framework(owners, bot_id).await;

    let mut client = Client::new(&bot_token)
        .event_handler(Handler)
        .framework(std_framework)
        .await?;

    // Block to define global data.
    // and so the data lock is not kept open in write mode.
    {
        // Open the data lock in write mode.
        let mut data = client.data.write().await;

        // Add the database connection to the data.
        {
            let mongo_uri = get_env::<String>("DATABASE_URL").expect("env::DATABASE_URL not set");
            let mongo_client = Mongo::with_uri_str(&mongo_uri).await?;
            data.insert::<MongoClient>(mongo_client.clone());
            info!("Database initialized");
        }

        // Add the shard manager to the data.
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));

        // Add the Voice Manager.
        data.insert::<VoiceManager>(Arc::clone(&client.voice_manager));

        // Set current time as the uptime.
        data.insert::<Uptime>(Instant::now());

        // Add the PlayerManager set.
        data.insert::<PlayerManager>(Arc::new(RwLock::new(HashMap::new())));
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start_autosharded().await {
        error!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}
