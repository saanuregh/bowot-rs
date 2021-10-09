use itconfig::*;
use lazy_static::lazy_static;

lazy_static! {
	// ENV vars
	pub static ref PREFIX: String = get_env_or_default("PREFIX", "!");
	pub static ref ENABLE_SERVICES: bool =
		get_env_or_default::<bool, bool>("ENABLE_SERVICES", true);

	// static values
	pub static ref STATUSES: Vec<[&'static str; 2]> = include_str!("static/statuses.txt")
		.split('\n')
		.map(|l| {
			let t = l.split(",").collect::<Vec<&'static str>>();
			[t[0], t[1]]
		})
		.collect();
	pub static ref SUBREDDIT_MEMES: Vec<&'static str> = include_str!("static/subreddit_memes.txt")
		.split('\n')
		.collect();
	pub static ref SHIP_RESPONSE: Vec<(&'static str, &'static str, &'static str)> =
		include_str!("static/ship.txt")
			.split('\n')
			.map(|l| {
				let t = l.split('|').collect::<Vec<&str>>();
				(t[0], t[1], t[2])
			})
			.collect();
	pub static ref PP_RESPONSE: Vec<&'static str> =
		include_str!("static/pp.txt").split('\n').collect();
}

pub const DAILY_AMOUNT: i64 = 1000;
pub const GAMBLE_MULTIPLIERS: [i64; 6] = [0, 1, 2, 3, 4, 5];
pub const GAMBLE_WEIGHTS: [f64; 5] = [6.0, 2.0, 1.7, 0.2, 0.1];
pub const MAX_DESCRIPTION_LENGTH: usize = 2048;
pub const DESCRIPTION_LENGTH_CUTOFF: usize = MAX_DESCRIPTION_LENGTH - 512;
pub const MAX_LIST_ENTRY_LENGTH: usize = 60;
pub const MAX_SINGLE_ENTRY_LENGTH: usize = 40;
pub const UNKNOWN_TITLE: &str = "Unknown title";
pub const LIVE_INDICATOR: &str = "🔴 **LIVE**";
