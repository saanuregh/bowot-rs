use crate::lang::SUBREDDIT_MEMES;
use rand::{seq::SliceRandom, thread_rng, Rng};
use regex;
use reqwest::Client;
use serde::Deserialize;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};

// Structs used to deserialize the output of the reddit api.
#[derive(Deserialize, Clone)]
pub struct RedditPost {
    pub title: String,
    pub subreddit_name_prefixed: String,
    pub selftext: String,
    pub downs: i64,
    pub ups: i64,
    pub created: f64,
    pub url: String,
    pub over_18: bool,
    pub permalink: String,
}
#[derive(Deserialize)]
struct RedditDataChild {
    data: RedditPost,
}

#[derive(Deserialize)]
struct RedditData {
    dist: i64,
    children: Vec<RedditDataChild>,
}

#[derive(Deserialize)]
struct RedditResponse {
    data: RedditData,
}

// Gets a random post from a vector of subreddit.
async fn random_post(subreddits: Vec<&str>, image: bool) -> Option<RedditPost> {
    let subreddit = subreddits.choose(&mut thread_rng()).unwrap();
    let client = Client::new();
    let re = regex::Regex::new(r"^.*(png|gif|jpeg|jpg)$").unwrap();
    let url = format!(
        r"https://www.reddit.com/r/{}/hot/.json?sort=top&t=week&limit=25",
        subreddit
    );
    if let Ok(resp) = client.get(&url).header("User-Agent", "bowot").send().await {
        if let Ok(data) = resp.json::<RedditResponse>().await {
            let posts = data.data.children;
            let mut rng = thread_rng();
            let mut idx: i64 = rng.gen_range(0, data.data.dist);
            let mut post: RedditPost;
            for _ in 0..10 {
                post = posts[idx as usize].data.clone();
                if !post.over_18 {
                    if image {
                        if re.is_match(&post.url) {
                            return Some(post);
                        }
                    } else {
                        if post.selftext != "" && post.selftext.len() < 2048 {
                            return Some(post);
                        }
                    }
                }
                idx = rng.gen_range(0, data.data.dist);
            }
        }
    }
    return None;
}

async fn reddit_command(
    ctx: &Context,
    msg: &Message,
    subreddits: Vec<&str>,
    image: bool,
) -> CommandResult {
    match random_post(subreddits, image).await {
        None => {
            msg.reply(ctx, "No result found.").await?;
        }
        Some(post) => {
            msg.channel_id
                .send_message(ctx, |m| {
                    m.reference_message(msg);
                    m.embed(|e| {
                        e.title(&post.title);
                        e.url(format!("https://www.reddit.com{}", &post.permalink));
                        if image {
                            e.image(&post.url);
                        } else {
                            e.description(&post.selftext);
                        }
                        e.footer(|f| {
                            f.text(format!(
                                "{} | 🔼: {} 🔽: {}",
                                &post.subreddit_name_prefixed, &post.ups, &post.downs
                            ))
                        })
                    })
                })
                .await?;
        }
    }
    Ok(())
}

/// Gets random meme from reddit.
#[command]
#[aliases(dank)]
#[bucket(reddit)]
async fn meme(ctx: &Context, msg: &Message) -> CommandResult {
    return reddit_command(ctx, msg, SUBREDDIT_MEMES.to_vec(), true).await;
}

/// Gets random image post from the subreddit given as argument.
///
/// Usage: `reddit_image dankmemes`
#[command]
#[bucket(reddit)]
#[aliases(rm)]
#[min_args(1)]
async fn reddit_image(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let subreddit = args.single_quoted::<String>()?;
    let subreddits = vec![subreddit.as_str()];
    return reddit_command(ctx, msg, subreddits, true).await;
}

/// Gets random text post from the subreddit given as argument.
///
/// Usage: `reddit_text copypasta`
#[command]
#[bucket(reddit)]
#[aliases(rt)]
#[min_args(1)]
async fn reddit_text(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let subreddit = args.single_quoted::<String>()?;
    let subreddits = vec![subreddit.as_str()];
    return reddit_command(ctx, msg, subreddits, false).await;
}
