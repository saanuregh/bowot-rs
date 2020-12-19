use crate::{constants::SUBREDDIT_MEMES, utils::apis::reddit_random_post};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};

async fn reddit_command(
    ctx: &Context,
    msg: &Message,
    subreddits: Vec<&str>,
    image: bool,
) -> CommandResult {
    let post = reddit_random_post(subreddits, image).await?;
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
                    f.icon_url(
                        "https://www.redditstatic.com/desktop2x/img/favicon/favicon-32x32.png",
                    );
                    f.text(format!(
                        "{} | 🔼: {} 🔽: {}",
                        &post.subreddit_name_prefixed, &post.ups, &post.downs
                    ))
                })
            })
        })
        .await?;
    Ok(())
}

/// Gets random meme from reddit.
#[command]
#[aliases(dank)]
#[bucket(reddit)]
async fn meme(ctx: &Context, msg: &Message) -> CommandResult {
    reddit_command(ctx, msg, SUBREDDIT_MEMES.to_vec(), true).await
}

/// Gets random image post from the subreddit given as argument.
///
/// Usage: `reddit_image dankmemes`
#[command]
#[bucket(reddit)]
#[aliases(rm)]
#[min_args(1)]
async fn reddit_image(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    reddit_command(
        ctx,
        msg,
        vec![args.single_quoted::<String>()?.as_str()],
        true,
    )
    .await
}

/// Gets random text post from the subreddit given as argument.
///
/// Usage: `reddit_text copypasta`
#[command]
#[bucket(reddit)]
#[aliases(rt)]
#[min_args(1)]
async fn reddit_text(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    reddit_command(
        ctx,
        msg,
        vec![args.single_quoted::<String>()?.as_str()],
        false,
    )
    .await
}
