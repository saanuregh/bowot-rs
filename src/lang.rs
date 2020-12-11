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
}
