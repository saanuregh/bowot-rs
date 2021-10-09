use std::{borrow::Cow, collections::HashMap, time::Duration};

use parse_duration;
use poise::{
    self, send_reply,
    serenity_prelude::{
        futures::StreamExt, AttachmentType, CacheHttp, Colour, Mentionable, ReactionType, User,
    },
};
use rand::{thread_rng, Rng};
use reqwest::Url;

use crate::{
    constants::{PP_RESPONSE, SHIP_RESPONSE},
    types::{Error, PoiseContext},
    utils::{
        apis::*,
        discord::{reply_embed, reply_plain},
        helpers::{capitalize_first, format_seconds},
    },
};

/// Defines a term, using the urban dictionary.
///
/// Usage: `urban lmao`
#[poise::command(slash_command, defer_response)]
pub async fn urban(
    ctx: PoiseContext<'_>,
    #[description = "Query to search for"] term: String,
) -> Result<(), Error> {
    let resp = urban_dict(term.to_string()).await?;
    if resp.list.is_empty() {
        reply_plain(ctx, format!("The term '{}' has no Urban Definitions", term)).await?;
    } else {
        let choice = &resp.list[0];
        let parsed_definition = &choice.definition.replace("[", "").replace("]", "");
        let parsed_example = &choice.example.replace("[", "").replace("]", "");

        if let Err(why) = reply_embed(ctx, |e| {
            e.title(&choice.word);
            e.url(&choice.permalink);
            e.description(format!(
                "submitted by **{}**\n:thumbsup: **{}** ┇ **{}** \
				 :thumbsdown:\n\n**Definition**\n{}\n\n**Example**\n{}",
                &choice.author,
                &choice.thumbs_up,
                &choice.thumbs_down,
                parsed_definition,
                parsed_example
            ));
            e.timestamp(choice.clone().written_on);
            e
        })
        .await
        {
            if "Embed too large." == why.to_string() {
                reply_plain(ctx, &choice.permalink).await?;
            } else {
                reply_plain(ctx, why).await?;
            }
        };
    }

    Ok(())
}

/// Translates a text to the specified language.
///
/// Usage:
/// Translate to japanese:
/// `translate ja Hello, World!`
///
/// Some supported languages:
/// ```
/// en -> English (Default)
/// es -> Spanish
/// fr -> French
/// it -> Italian
/// de -> German
/// pt -> Brazilian Portuguese
/// ja -> Japanese
/// ko -> Korean
/// zh -> Chinese (Simplified)
/// hi -> Hindi
/// ru -> Russian
/// ar -> Arabic
/// tr -> Turkish
/// ```
///
/// For full list of supported languages refer: https://cloud.google.com/translate/docs/languages
#[poise::command(slash_command, defer_response)]
pub async fn translate(
    ctx: PoiseContext<'_>,
    #[description = "Target language"] target: String,
    #[description = "What do you wanna translate?"] text: String,
) -> Result<(), Error> {
    let translated = get_translate(&target, &text).await?;
    reply_plain(ctx, translated).await?;

    Ok(())
}

/// Searches a term on duckduckgo.com, for you.
///
/// Usage: `ddg hello world`
#[poise::command(slash_command)]
pub async fn duck_duck_go(
    ctx: PoiseContext<'_>,
    #[description = "Query to search for"] term: String,
) -> Result<(), Error> {
    let url = Url::parse_with_params("https://lmddgtfy.net/", &[("q", term)])?;
    reply_plain(ctx, url).await?;

    Ok(())
}

/// Shows the information of a user.
/// (not bound to a guild)
#[poise::command(slash_command)]
pub async fn profile(
    ctx: PoiseContext<'_>,
    #[description = "Pick a user or give nothing to pick yourself"] user: Option<User>,
) -> Result<(), Error> {
    let user = user.as_ref().unwrap_or(ctx.author());
    reply_embed(ctx, |e| {
        if user.bot {
            e.title(format!("[BOT] {}", user.tag(),));
        } else {
            e.title(user.tag());
        }

        e.field("ID:", user.id.0, false);
        e.field(
            "Created at:",
            format!(
                "{}UTC\n({} ago)",
                user.created_at().to_rfc2822().replace("+0000", ""),
                {
                    let time = chrono::Utc::now().timestamp() - user.created_at().timestamp();
                    format_seconds(time as u64)
                }
            ),
            false,
        );

        e.image(user.face())
    })
    .await?;

    Ok(())
}

// Structs used to create reaction based polls.
struct Poll {
    emoji: char,
    option: String,
    votes: u64,
}

impl Poll {
    fn new(emoji: char, option: &str, votes: u64) -> Poll {
        Poll {
            emoji,
            option: option.to_string(),
            votes,
        }
    }
}

/// Create a poll.
///
/// ```
/// s -> Second
/// m -> Minute
/// ```
///
/// Usage:
/// `poll 2m title option1,option2,option3`
/// `poll "1m 30s" "title long" option1,option2,option3`
/// Duration between 10 sec and 2 minutes
#[poise::command(slash_command)]
pub async fn poll(
    ctx: PoiseContext<'_>,
    #[description = "Duration"] unformatted_time: String,
    #[description = "Title"] title: String,
    #[description = "Options seperated by commas"] unformatted_options: String,
) -> Result<(), Error> {
    let seconds = parse_duration::parse(&unformatted_time)?.as_secs();
    let options = unformatted_options.split(",").collect::<Vec<&str>>();
    let error_msg = {
        if seconds < 10 {
            "Duration is too short, stay within 10 sec to 2 mins"
        } else if seconds > 120 {
            "Duration is too high, stay within 10 sec to 2 mins"
        } else if unformatted_options.is_empty() {
            "Poll options are not provided"
        } else if options.len() < 2 {
            "Requires alteast 2 options"
        } else {
            ""
        }
    };
    if !error_msg.is_empty() {
        reply_plain(ctx, error_msg).await?;

        return Ok(());
    }
    let reactions = vec![
        '🇦', '🇧', '🇨', '🇩', '🇪', '🇫', '🇬', '🇭', '🇮', '🇯', '🇰', '🇱', '🇲', '🇳', '🇴', '🇵', '🇶', '🇷',
        '🇸', '🇹',
    ];
    let mut polls: Vec<Poll> = Vec::new();
    for (i, option) in options.iter().enumerate() {
        polls.push(Poll::new(reactions[i], option, 0));
    }
    let poll_msg = reply_embed(ctx, |e| {
        e.title(capitalize_first(&title));
        let mut text_definitions = String::new();
        for p in polls.iter() {
            text_definitions += &format!("{} - **{}**\n", p.emoji, capitalize_first(&p.option));
        }
        e.description(&text_definitions);
        e.footer(|f| {
            f.text(format!(
                "Vote by reacting to the emojis, you have {} to vote",
                unformatted_time
            ))
        })
    })
    .await?
    .message()
    .await?;

    for p in polls.iter() {
        poll_msg.react(ctx.discord(), p.emoji).await?;
    }
    let http = ctx.discord().http();
    let channel_id = poll_msg.channel_id.0 as u64;
    let message_id = poll_msg.id.0 as u64;
    let mut user_reactions: HashMap<u64, ReactionType> = HashMap::new();
    let mut collector = poll_msg
        .await_reactions(ctx.discord())
        .timeout(Duration::from_secs(seconds))
        .await;
    while let Some(reaction_action) = collector.next().await {
        let reaction = reaction_action.as_inner_ref();
        let user_id = reaction.user_id.unwrap().0 as u64;
        let emoji = reaction.emoji.clone();
        let mut flag = false;
        for r in reactions.clone() {
            if r.to_string() == emoji.as_data() {
                flag = true;
                break;
            }
        }
        if !flag {
            http.delete_reaction(channel_id, message_id, Some(user_id), &emoji)
                .await?;
            continue;
        }
        if user_reactions.contains_key(&user_id) {
            if user_reactions[&user_id].as_data() != emoji.as_data() {
                http.delete_reaction(
                    channel_id,
                    message_id,
                    Some(user_id),
                    &user_reactions[&user_id],
                )
                .await?;
            }
        }
        user_reactions.insert(user_id, emoji);
    }
    let mut new_poll_msg = ctx
        .discord()
        .http()
        .get_message(channel_id, message_id)
        .await?;
    let mut total_votes = 0;
    for p in polls.iter_mut() {
        for mr in &new_poll_msg.reactions {
            if mr.reaction_type.as_data() == p.emoji.to_string() {
                p.votes = mr.count - 1;
                total_votes += p.votes
            }
        }
    }
    new_poll_msg.delete_reactions(ctx.discord().http()).await?;
    let mut text_definitions = "Nobody voted".to_string();
    if total_votes > 0 {
        text_definitions = "".to_string();
        for p in polls.iter() {
            text_definitions += &format!(
                "{} - **{}** - {}%\n",
                p.emoji,
                capitalize_first(&p.option),
                p.votes / total_votes * 100
            );
        }
    }
    new_poll_msg
        .edit(ctx.discord().http(), |m| {
            m.embed(|embed| {
                embed.title(capitalize_first(&title));
                embed.description(&text_definitions)
            })
        })
        .await?;

    Ok(())
}

/// Get a random Chuck Norris joke.
#[poise::command(slash_command, defer_response)]
pub async fn chuck(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let resp = get_chuck().await?;
    reply_plain(
        ctx,
        resp.value
            .unwrap_or("Chuck's a little busy here, try again later!".to_string()),
    )
    .await?;

    Ok(())
}

/// Throw a dice.
#[poise::command(slash_command)]
pub async fn dice(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let n: i64 = thread_rng().gen_range(1..7);
    reply_plain(ctx, format!("You rolled a {}", n)).await?;

    Ok(())
}

/// Uwufy a text.
///
/// Usage:
/// `uwufy hello world`
#[poise::command(slash_command)]
pub async fn uwufy(
    ctx: PoiseContext<'_>,
    #[description = "Sup?"] message: String,
) -> Result<(), Error> {
    let raw_words = message.split(' ');
    let mut words = Vec::new();
    for word in raw_words {
        match word {
            "you" => words.push(word.to_string()),
            "uwu" => words.push(word.to_string()),
            "owo" => words.push(word.to_string()),
            "one" => words.push("wone".to_string()),
            "two" => words.push("two".to_string()),
            "three" => words.push("thwee".to_string()),
            "lewd" => words.push("lewd".to_string()),
            "cute" => words.push("cwute".to_string()),
            _ => {
                if word.len() > 2 {
                    let mut w = word.to_string();
                    w = w.replace("our", "\u{200b}w");
                    w = w.replace("r", "w");
                    w = w.replace("R", "W");
                    w = w.replace("l", "w");
                    w = w.replace("L", "W");
                    w = w.replace("ar", " ");
                    w = w.replace("ai", "+");
                    w = w.replace("a", "wa");
                    w = w.replace("wawa", "waa");
                    w = w.replace(" ", "aw");
                    w = w.replace("ie", " ");
                    w = w.replace("i", "wi");
                    w = w.replace("wiwi", "wii");
                    w = w.replace(" ", "ie");
                    w = w.replace("+", "ai");
                    w = w.replace("ge", " ");
                    w = w.replace("ke", "+");
                    w = w.replace("e", "we");
                    w = w.replace("wewe", "wee");
                    w = w.replace(" ", "ge");
                    w = w.replace("+", "ke");
                    w = w.replace("ou", "=");
                    w = w.replace("cho", " ");
                    w = w.replace("o", "wo");
                    w = w.replace("wowo", "woo");
                    w = w.replace(" ", "cho");
                    w = w.replace("gu", " ");
                    w = w.replace("qu", "+");
                    w = w.replace("u", "wu");
                    w = w.replace("wuwu", "wuu");
                    w = w.replace(" ", "gu");
                    w = w.replace("+", "qu");
                    w = w.replace("=", "ouw");
                    if !word.starts_with("A") {
                        w = w.replace("A", "WA");
                    } else {
                        w = w.replace("A", "Wa");
                    }
                    if !word.starts_with("E") {
                        w = w.replace("E", "WE");
                    } else {
                        w = w.replace("E", "We");
                    }
                    if !word.starts_with("I") {
                        w = w.replace("I", "WI");
                    } else {
                        w = w.replace("I", "Wi");
                    }
                    if !word.starts_with("O") {
                        w = w.replace("O", "WO");
                    } else {
                        w = w.replace("O", "Wo");
                    }
                    if !word.starts_with("U") {
                        w = w.replace("U", "WU");
                    } else {
                        w = w.replace("U", "Wu");
                    }
                    w = w.replace("\u{200b}", "ouw");
                    w = w.replace("@", "@\u{200b}");

                    words.push(w);
                } else {
                    words.push(word.to_string());
                }
            }
        }
    }
    words.push("uwu".to_string());
    let mut m = words.join(" ");
    m = m.replace("ww", "w");
    m = m.replace("Ww", "W");
    m = m.replace("WW", "W");
    reply_plain(ctx, &m).await?;

    Ok(())
}

/// Get a random fact.
#[poise::command(slash_command, defer_response)]
pub async fn fact(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let resp = neko_api("fact", false).await?;
    reply_plain(
        ctx,
        resp.get("fact")
            .unwrap_or(&"Couldn't find a fact, try again later!".to_string()),
    )
    .await?;

    Ok(())
}

/// Why?.
#[poise::command(slash_command, defer_response)]
pub async fn why(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let resp = neko_api("why", false).await?;
    reply_plain(ctx, resp.get("why").unwrap_or(&"Why".to_string())).await?;

    Ok(())
}

/// Eightball.
#[poise::command(slash_command, defer_response)]
pub async fn eightball(
    ctx: PoiseContext<'_>,
    #[description = "Sup?"] _message: Option<String>,
) -> Result<(), Error> {
    let resp = neko_api("8ball", false).await?;
    if let Some(response) = resp.get("response") {
        if let Some(url) = resp.get("url") {
            reply_embed(ctx, |e| {
                e.title(response);
                e.image(url)
            })
            .await?;

            return Ok(());
        }
    }
    reply_plain(ctx, "Lost the eightball, try again later!").await?;

    Ok(())
}

/// Ship two person
///
/// Usage:
/// `ship <@user1> <@user2>`
#[poise::command(slash_command)]
pub async fn ship(
    ctx: PoiseContext<'_>,
    #[description = "Ship who?"] user1: User,
    #[description = "With?"] user2: User,
) -> Result<(), Error> {
    let ship_response = &*SHIP_RESPONSE;

    let percentage = (user1.id.0 + user2.id.0) % 101;
    let idx = (percentage / 10) as usize;
    let (emoji, title, verdict) = ship_response[idx];
    let color = Colour::from_rgb(
        {
            if percentage > 50 {
                (255 - ((percentage - 50) * 2 * 255) / 100) as u8
            } else {
                255
            }
        },
        {
            if percentage < 50 {
                ((percentage * 2 * 255) / 100) as u8
            } else {
                255
            }
        },
        0,
    );
    send_reply(ctx, |m| {
        m.embed(|e| {
            e.title(title);
            e.description(format!(
                "{}\n\n{}\n\n{} and {} compatibility reading is at **{}**%",
                emoji,
                verdict,
                user1.mention(),
                user2.mention(),
                percentage
            ));
            e.color(color)
        })
    })
    .await?;

    Ok(())
}

/// Rate PP
///
/// Usage:
/// `pp <@user>`
#[poise::command(slash_command)]
pub async fn pp(
    ctx: PoiseContext<'_>,
    #[description = "Give a user or pick yourself"] user: Option<User>,
) -> Result<(), Error> {
    let verdict = &*PP_RESPONSE;
    let user = user.as_ref().unwrap_or(ctx.author());
    let length = user.id.0 % 101;
    let bot_name = ctx.discord().cache.current_user().name.clone();
    reply_embed(ctx, |e| {
        e.title(format!("Dr {}'s PP report", bot_name));
        e.description(format!(
            "Patient: {}\nScan: 8{}D\nVerdict: {}\n\nSigned by,\n_Dr {}_",
            user.mention(),
            "=".repeat((length / 10) as usize),
            verdict[(length / (100 / (verdict.len() - 1)) as u64) as usize],
            bot_name
        ))
    })
    .await?;

    Ok(())
}

/// F
///
/// Usage:
/// `respect soldier`
#[poise::command(slash_command)]
pub async fn respect(
    ctx: PoiseContext<'_>,
    #[description = "F for?"] message: String,
) -> Result<(), Error> {
    reply_embed(ctx, |e| {
        e.description(format!(
            "{} has paid their respect to {}.",
            ctx.author().mention(),
            message
        ));
        e.footer(|f| f.text("Press F to pay respect."))
    })
    .await?
    .message()
    .await?
    .react(ctx.discord(), '🇫')
    .await?;

    Ok(())
}

/// Generate triggered gif using avatar
///
/// Usage:
/// `triggered`
/// `triggered <@user>`
#[poise::command(slash_command, defer_response)]
pub async fn triggered(
    ctx: PoiseContext<'_>,
    #[description = "Give a user or pick yourself"] user: Option<User>,
) -> Result<(), Error> {
    let user = user.as_ref().unwrap_or(ctx.author());
    let image = generate_triggered_avatar(
        user.static_avatar_url()
            .unwrap_or(user.default_avatar_url())
            .replace(".webp?size=1024", ".png"),
    )
    .await?;

    ctx.channel_id()
        .send_message(ctx.discord(), |m| {
            m.add_file(AttachmentType::Bytes {
                data: Cow::from(image),
                filename: format!("triggered-{}.gif", user.id),
            })
        })
        .await?;

    Ok(())
}
