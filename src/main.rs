mod commands;
mod database;
mod framework;
mod handler;
mod service;
mod utils;

use database::*;
use framework::*;
use handler::Handler;

use dotenv;
use std::{
    clone::Clone,
    collections::HashSet,
    convert::TryInto,
    fs::File,
    io::prelude::*,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tracing::{error, info, instrument, Level};
use tracing_log::LogTracer;
use tracing_subscriber::{EnvFilter, FmtSubscriber};
//use tracing_futures::Instrument;
//use log;

use lavalink_rs::{gateway::LavalinkEventHandler, LavalinkClient};
use serenity::{
    async_trait,
    client::{
        bridge::{gateway::ShardManager, voice::ClientVoiceManager},
        Client,
    },
    http::Http,
    model::id::GuildId,
    prelude::{RwLock, TypeMapKey},
};
use toml::Value;
use wither::mongodb::Client as Mongo;

// Defining the structures to be used for "global" data
// this data is not really global, it's just shared with Context.data
struct ShardManagerContainer; // Shard manager to use for the latency.
struct MongoClient; // The connection to the mongo database.
struct Config; // For the configuration found on "config.toml"
struct Uptime; //  This is for the startup time of the bot.
struct VoiceManager; //  This is the struct for the voice manager.
struct VoiceGuildUpdate; //  Hashset of guilds having active voice connection.
struct Lavalink; //  This is the struct for the lavalink client.
struct LavalinkHandler;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

impl TypeMapKey for MongoClient {
    type Value = Mongo;
}

impl TypeMapKey for Config {
    type Value = Value;
}

impl TypeMapKey for Uptime {
    type Value = Instant;
}

impl TypeMapKey for VoiceGuildUpdate {
    type Value = Arc<RwLock<HashSet<GuildId>>>;
}

impl TypeMapKey for VoiceManager {
    type Value = Arc<Mutex<ClientVoiceManager>>;
}

impl TypeMapKey for Lavalink {
    type Value = Arc<Mutex<LavalinkClient>>;
}

#[async_trait]
impl LavalinkEventHandler for LavalinkHandler {}

#[tokio::main(core_threads = 8)]
#[instrument]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut file = File::open("config.toml").expect("Configuration file not found");
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let configuration = contents.parse::<Value>().unwrap();
    let bot_config = configuration
        .get("bot")
        .expect("Can't find bot object in configuration file");
    if let Some(t) = bot_config.get("enable_tracing") {
        if t.as_bool().unwrap_or(false) {
            LogTracer::init()?;
            let mut base_level = "info";
            if let Some(l) = bot_config.get("trace_level") {
                base_level = l.as_str().expect("Invalid value for trace_level");
            }
            let level = match base_level {
                "error" => Level::ERROR,
                "warn" => Level::WARN,
                "info" => Level::INFO,
                "debug" => Level::DEBUG,
                "trace" => Level::TRACE,
                _ => Level::INFO,
            };
            info!("Tracer initialized.");
            if let Ok(_) = dotenv::dotenv() {
                let subscriber = FmtSubscriber::builder()
                    .with_env_filter(EnvFilter::from_default_env())
                    .finish();
                tracing::subscriber::set_global_default(subscriber)?;
            } else {
                let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
                tracing::subscriber::set_global_default(subscriber)?;
            };
            info!("Subscriber initialized.");
        }
    }
    let bot_token = bot_config
        .get("token")
        .expect("Requires bot token in configuration file")
        .as_str()
        .expect("Could't get bot token from configuration file, invalid format");

    let http = Http::new_with_token(&bot_token);
    let (owners, bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);
            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    let std_framework = get_framework(owners).await;

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
            let mongo_uri = configuration
                .get("database")
                .expect("Can't find database object in configuration file")
                .get("uri")
                .expect("Requires mongo uri in configuration file")
                .as_str()
                .expect("Could't get mongo uri from configuration file, invalid format");
            let mongo_client = Mongo::with_uri_str(mongo_uri).await?;
            data.insert::<MongoClient>(mongo_client.clone());
            info!("Database initialized");
        }

        // Add the shard manager to the data.
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));

        // Add the Config to the data.
        data.insert::<Config>(configuration.clone());

        // Add the Voice Manager.
        data.insert::<VoiceManager>(Arc::clone(&client.voice_manager));

        // Set current time as the uptime.
        data.insert::<Uptime>(Instant::now());

        // Add the VoiceGuild set.
        data.insert::<VoiceGuildUpdate>(Arc::new(RwLock::new(HashSet::new())));

        // Initialize and add Lavalink.
        {
            let host = configuration["lavalink"]["host"].as_str().unwrap();
            let port = configuration["lavalink"]["port"].as_integer().unwrap();
            let password = configuration["lavalink"]["password"].as_str().unwrap();

            let mut counter: i32 = 0;
            loop {
                counter += 1;
                if counter == 10 {
                    panic!("Could not connect to lavalink after 10 tries, exiting!")
                }
                let mut lava_client = LavalinkClient::new(bot_id);
                lava_client.set_host(host.to_string());
                lava_client.set_password(password.to_string());
                lava_client.set_port(port.try_into().unwrap());
                match lava_client.initialize(LavalinkHandler).await {
                    Ok(lava) => {
                        data.insert::<Lavalink>(lava);
                        info!("Lavalink initialized");
                        break;
                    }
                    Err(why) => {
                        error!("Could not connect to lavalink, retrying: {:?}", why);
                        tokio::time::delay_for(Duration::from_secs(5)).await;
                    }
                }
            }
        }
    }

    // start listening for events by starting a single shard
    if let Err(why) = client.start_autosharded().await {
        error!("An error occurred while running the client: {:?}", why);
    }

    Ok(())
}
