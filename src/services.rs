use std::sync::Arc;

use poise::serenity_prelude::{Activity, OnlineStatus};
use rand::seq::SliceRandom;
use tokio::time::Duration;
use tracing::info;

use crate::{constants::STATUSES, types::SerenityContext};

async fn status_update(ctx: Arc<SerenityContext>) {
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

pub async fn start_services(ctx: Arc<SerenityContext>) {
	let ctx_clone1 = Arc::clone(&ctx);
	tokio::spawn(async move {
		loop {
			tokio::join!(status_update(Arc::clone(&ctx_clone1)));
			tokio::time::sleep(Duration::from_secs(1800)).await;
		}
	});
}
