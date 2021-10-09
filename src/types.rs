use std::{collections::HashMap, error, sync::Arc};

use poise::{
    serenity,
    serenity_prelude::{ChannelId, RwLock},
};
use tokio::time::Instant;

use crate::data::Data;

pub type Error = Box<dyn error::Error + Send + Sync>;
pub type PoiseContext<'a> = poise::Context<'a, Data, Error>;
pub type SerenityContext = serenity::client::Context;

pub type LastMessageHashMap = Arc<RwLock<HashMap<u64, ChannelId>>>;
pub type IdleHashMap = Arc<RwLock<HashMap<u64, Instant>>>;
