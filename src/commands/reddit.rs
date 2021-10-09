use crate::{
	constants::SUBREDDIT_MEMES,
	types::{Error, PoiseContext},
	utils::{apis::reddit_random_post, discord::reply_embed},
};

async fn reddit_command(
	ctx: PoiseContext<'_>,
	subreddits: &[&str],
	image: bool,
) -> Result<(), Error> {
	let post = reddit_random_post(subreddits, image).await?;
	reply_embed(ctx, |e| {
		e.title(&post.title);
		e.url(format!("https://www.reddit.com{}", &post.permalink));
		if image {
			e.image(&post.url);
		} else {
			e.description(&post.selftext);
		}
		e.footer(|f| {
			f.icon_url("https://www.redditstatic.com/desktop2x/img/favicon/favicon-32x32.png");
			f.text(format!(
				"{} | 🔼: {} 🔽: {}",
				&post.subreddit_name_prefixed, &post.ups, &post.downs
			))
		})
	})
	.await?;

	Ok(())
}

/// Gets random meme from reddit.
#[poise::command(slash_command, defer_response)]
pub async fn meme(ctx: PoiseContext<'_>) -> Result<(), Error> {
	reddit_command(ctx, &SUBREDDIT_MEMES, true).await
}

/// Gets random image post from the subreddit given as argument.
///
/// Usage: `reddit_image dankmemes`
#[poise::command(slash_command, defer_response)]
pub async fn reddit_image(
	ctx: PoiseContext<'_>,
	#[description = "Subreddit"] subreddit: String,
) -> Result<(), Error> {
	reddit_command(ctx, &[&subreddit], true).await
}

/// Gets random text post from the subreddit given as argument.
///
/// Usage: `reddit_text copypasta`
#[poise::command(slash_command, defer_response)]
pub async fn reddit_text(
	ctx: PoiseContext<'_>,
	#[description = "Subreddit"] subreddit: String,
) -> Result<(), Error> {
	reddit_command(ctx, &[&subreddit], false).await
}
