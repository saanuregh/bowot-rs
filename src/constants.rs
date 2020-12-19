use itconfig::*;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref STATUSES: Vec<[&'static str; 2]> = include_str!("lang/statuses.txt")
        .split('\n')
        .map(|l| {
            let t = l.split(",").collect::<Vec<&'static str>>();
            [t[0], t[1]]
        })
        .collect();
    pub static ref HYDRATE: Vec<&'static str> =
        include_str!("lang/hydrate.txt").split('\n').collect();
    pub static ref SUBREDDIT_MEMES: Vec<&'static str> = include_str!("lang/subreddit_memes.txt")
        .split('\n')
        .collect();
    pub static ref DEFAULT_PREFIX: String = get_env_or_default("PREFIX", "!");
    pub static ref ENABLE_SERVICES: bool =
        get_env_or_default::<bool, bool>("ENABLE_SERVICES", true);
    pub static ref TRACING: bool = get_env_or_default::<bool, bool>("TRACING", false);
    pub static ref TRACE_LEVEL: &'static str = get_env_or_default("TRACE_LEVEL", "info");
    pub static ref DATABASE: &'static str = get_env_or_default("DATABASE", "bowot");
    pub static ref PORT: u16 = get_env_or_default::<u16, u16>("PORT", 80);
}
