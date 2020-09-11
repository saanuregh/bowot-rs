use crate::{
    utils::{basic_functions::*, valorant::*},
    Guild, MongoClient,
};
use comfy_table::{Cell, CellAlignment::Center, ContentArrangement::Dynamic, Table};
use fasteval::error::Error;
use qrcode::{render::unicode, QrCode};
use rand::{thread_rng, Rng};
use reqwest::{Client as ReqwestClient, Url};
use serde::Deserialize;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    futures::stream::StreamExt,
    model::channel::Message,
    model::id::UserId,
    prelude::Context,
};
use std::{collections::HashMap, time::Duration};

// Structs used to deserialize the output of the urban dictionary api call.
#[derive(Deserialize, Clone)]
struct UrbanDict {
    definition: String,
    permalink: String,
    thumbs_up: u32,
    thumbs_down: u32,
    author: String,
    written_on: String,
    example: String,
    word: String,
}

#[derive(Deserialize)]
struct UrbanList {
    list: Vec<UrbanDict>,
}

// Structs used to deserialize the output of the dictionary api call.
#[derive(Debug, Deserialize)]
struct DictionaryElement {
    word: String,
    phonetic: Option<String>,
    origin: Option<String>,
    meanings: Vec<Meaning>,
}

#[derive(Debug, Deserialize)]
struct Meaning {
    #[serde(rename = "partOfSpeech")]
    part_of_speech: Option<String>,
    definitions: Vec<Definition>,
}

#[derive(Debug, Deserialize)]
struct Definition {
    definition: String,
    synonyms: Option<Vec<String>>,
    example: Option<String>,
}

// Structs used to deserialize the output of the chuck norris joke api call.
#[derive(Debug, Deserialize)]
struct ChuckResponse {
    categories: Option<Vec<String>>,
    value: Option<String>,
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
            emoji: emoji,
            option: option.to_string(),
            votes: votes,
        }
    }
}

/// Sends a qr code of the term mentioned.
///
/// Usage: `qr Hello world!`
#[command]
#[min_args(1)]
async fn qr(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let words = args.message();
    let code = QrCode::new(words)?;
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();
    msg.channel_id
        .say(ctx, format!(">>> ```{}```", image))
        .await?;
    Ok(())
}

/// Defines a term, using the urban dictionary.
///
/// Usage: `urban lmao`
#[command]
#[aliases(
    udic,
    udefine,
    define_urban,
    defineurban,
    udict,
    udictonary,
    urban_dictionary,
    u_dictionary,
    u_define,
    urban_define,
    define_urban
)]
#[min_args(1)]
async fn urban(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let term = args.message();
    let url = Url::parse_with_params(
        "http://api.urbandictionary.com/v0/define",
        &[("term", term)],
    )?;
    let reqwest = ReqwestClient::new();
    let resp = reqwest.get(url).send().await?.json::<UrbanList>().await?;
    if resp.list.is_empty() {
        msg.channel_id
            .say(ctx, format!("The term '{}' has no Urban Definitions", term))
            .await?;
    } else {
        let choice = &resp.list[0];
        let parsed_definition = &choice.definition.replace("[", "").replace("]", "");
        let parsed_example = &choice.example.replace("[", "").replace("]", "");
        let mut fields = vec![("Definition", parsed_definition, false)];
        if parsed_example != &"".to_string() {
            fields.push(("Example", parsed_example, false));
        }

        if let Err(why) = msg
            .channel_id
            .send_message(ctx, |m| {
                m.embed(|e| {
                    e.title(&choice.word);
                    e.url(&choice.permalink);
                    e.description(format!(
                        "submitted by **{}**\n\n:thumbsup: **{}** ┇ **{}** :thumbsdown:\n",
                        &choice.author, &choice.thumbs_up, &choice.thumbs_down
                    ));
                    e.fields(fields);
                    e.timestamp(choice.clone().written_on);
                    e
                });
                m
            })
            .await
        {
            if "Embed too large." == why.to_string() {
                msg.channel_id.say(ctx, &choice.permalink).await?;
            } else {
                msg.channel_id.say(ctx, why).await?;
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
#[command]
#[aliases(trans)]
#[min_args(2)]
async fn translate(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let target = args.single::<String>()?;
    let text = args.rest();
    let url = Url::parse_with_params(
        "https://translate.googleapis.com/translate_a/single",
        &[
            ("client", "gtx"),
            ("ie", "UTF-8"),
            ("oe", "UTF-8"),
            ("dt", "t"),
            ("sl", "auto"),
            ("tl", &target),
            ("q", &text),
        ],
    )?;
    let reqwest = ReqwestClient::new();
    let resp = reqwest.get(url).send().await?.text().await?;
    let idx = resp.find(&format!("\",\"{}\"", text)).unwrap();
    let translated = resp.get(4..idx).unwrap();
    msg.channel_id
        .send_message(ctx, |m| m.content(translated))
        .await?;
    Ok(())
}

/// Searches a term on duckduckgo.com, for you.
///
/// Usage: `ddg hello world`
#[command]
#[min_args(1)]
#[aliases(ddg, duck, duckduckgo, search, better_than_google, betterthangoogle)]
async fn duck_duck_go(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let url = Url::parse_with_params("https://lmddgtfy.net/", &[("q", args.message())])?;
    msg.channel_id.say(ctx, url).await?;

    Ok(())
}

/// Shows the information of a user.
/// (not bound to a guild)
#[command]
#[aliases(pfp, avatar, discord_profile, prof, user, u)]
async fn profile(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let user = if let Ok(user_id) = args.single_quoted::<UserId>() {
        user_id.to_user(ctx).await?
    } else {
        msg.author.clone()
    };

    msg.channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
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
                            let date = chrono::Utc::now();
                            let time = date.timestamp() - user.created_at().timestamp();
                            let duration = Duration::from_secs(time as u64);
                            humantime::format_duration(duration)
                        }
                    ),
                    false,
                );

                e.image(user.face())
            })
        })
        .await?;

    Ok(())
}

/// Calculates an expression.
///
/// Example: `calc 1+2*3/4^5%6 + log(100K) + log(e(),100) + [3*(3-3)/3] + (2<3) && 1.23`
///
/// The precise integer limit is the signed 32 bit integer (-2147483648 to 2147483647)
/// The the unprecise integer limit is almost the signed 1024 bit integer.
/// The floating point precision is 64 bit.
///
/// Supported operators:
/// ```
/// +               Addition
/// -               Subtraction
/// *               Multiplication
/// /               Division
/// %               Modulo
/// ^ **            Exponentiation
/// && (and)        Logical AND with short-circuit
/// || (or)         Logical OR with short-circuit
/// == != < <= >= > Comparisons (all have equal precedence)
///
/// ---------------
///
/// Integers: 1, 2, 10, 100, 1001
///
/// Decimals: 1.0, 1.23456, 0.000001
///
/// Exponents: 1e3, 1E3, 1e-3, 1E-3, 1.2345e100
///
/// Suffix:
/// 1.23p       = 0.00000000000123
/// 1.23n       = 0.00000000123
/// 1.23µ 1.23u = 0.00000123
/// 1.23m       = 0.00123
/// 1.23K 1.23k = 1230
/// 1.23M       = 1230000
/// 1.23G       = 1230000000
/// 1.23T       = 1230000000000
///
/// ---------------
///
/// e()  -- Euler's number (2.718281828459045)
/// pi() -- π (3.141592653589793)
///
/// log(base=10, val)
/// ---
/// Logarithm with optional 'base' as first argument.
/// If not provided, 'base' defaults to '10'.
/// Example: "log(100) + log(e(), 100)"
///
/// int(val)
/// ceil(val)
/// floor(val)
/// round(modulus=1, val)
/// ---
/// Round with optional 'modulus' as first argument.
/// Example: "round(1.23456) == 1  &&  round(0.001, 1.23456) == 1.235"
///
/// sqrt(val)
/// abs(val)
/// sign(val)
///
/// min(val, ...) -- Example: "min(1, -2, 3, -4) == -4"
/// max(val, ...) -- Example: "max(1, -2, 3, -4) == 3"
///
/// sin(radians)     asin(val)
/// cos(radians)     acos(val)
/// tan(radians)     atan(val)
/// sinh(val)        asinh(val)
/// cosh(val)        acosh(val)
/// tanh(val)        atanh(val)
/// ```
#[command]
#[aliases(calc, math, maths)]
#[min_args(1)]
async fn calculator(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut operation = args.message().to_string();
    operation = operation.replace("**", "^");
    operation = operation.replace("pi()", "pi");
    operation = operation.replace("pi", "pi()");
    operation = operation.replace("π", "pi()");
    operation = operation.replace("euler", "e()");

    let mut operation_without_markdown = operation.replace(r"\\", r"\\\\");
    // " my ide is bugged lol

    for i in &["*", "`", "_", "~", "|"] {
        operation_without_markdown = operation_without_markdown.replace(i, &format!(r"\{}", i));
    }

    let mut cb = |name: &str, args: Vec<f64>| -> Option<f64> {
        match name {
            "sqrt" => {
                let a = args.get(0);
                if let Some(x) = a {
                    let l = x.log10();
                    Some(10.0_f64.powf(l / 2.0))
                } else {
                    None
                }
            }
            _ => None,
        }
    };

    let val = fasteval::ez_eval(&operation, &mut cb);

    match val {
        Err(why) => {
            let text = match &why {
                Error::SlabOverflow => "Too many Expressions/Values/Instructions were stored in the Slab.".to_string(),
                Error::EOF => "Reached an unexpected End Of Input during parsing.\nMake sure your operators are complete.".to_string(),
                Error::EofWhileParsing(x) => format!("Reached an unexpected End Of Input during parsing:\n{}", x),
                Error::Utf8ErrorWhileParsing(_) => "The operator could not be decoded with UTF-8".to_string(),
                Error::TooLong => "The expression is too long.".to_string(),
                Error::TooDeep => "The expression is too recursive.".to_string(),
                Error::UnparsedTokensRemaining(x) => format!("An expression was parsed, but there is still input data remaining.\nUnparsed data: {}", x),
                Error::InvalidValue => "A value was expected, but invalid input data was found.".to_string(),
                Error::ParseF64(x) => format!("Could not parse a 64 bit floating point number:\n{}", x),
                Error::Expected(x) => format!("The expected input data was not found:\n{}", x),
                Error::WrongArgs(x) => format!("A function was called with the wrong arguments:\n{}", x),
                Error::Undefined(x) => format!("The expression tried to use an undefined variable or function, or it didn't provide any required arguments.:\n{}", x),
                Error::Unreachable => "This error should never happen, if it did, contact nitsuga5124#2207 immediately!".to_string(),
                _ => format!("An unhandled error occurred:\n{:#?}", &why),
            };

            msg.channel_id
                .send_message(ctx, |m| {
                    m.embed(|e| {
                        e.title("ERROR");
                        e.description(text);
                        e.field("Operation", &operation_without_markdown, true);
                        e.footer(|f| f.text(format!("Submitted by: {}", msg.author.tag())))
                    })
                })
                .await?;
        }
        Ok(res) => {
            msg.channel_id
                .send_message(ctx, |m| {
                    m.embed(|e| {
                        e.title("Result");
                        e.description(res);
                        e.field("Operation", &operation_without_markdown, true);
                        e.footer(|f| f.text(format!("Submitted by: {}", msg.author.tag())))
                    })
                })
                .await?;
        }
    }
    Ok(())
}

/// Gives the definition of a word.
///
/// Usage:
/// `define hello`
/// `define ja こんにちは`
///
/// Supported languages:
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
#[command]
#[aliases(dict, define)]
#[min_args(1)]
async fn dictionary(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let supported_lang = vec![
        "en", "es", "fr", "ja", "ru", "de", "it", "ko", "ar", "tr", "zh", "hi", "pt",
    ];
    let mut lang = args.single_quoted::<String>()?;
    let mut word = lang.clone();
    if lang == "kr" {
        lang = "ko".to_string();
    }
    if lang == "br" {
        lang = "pt".to_string();
    }
    if supported_lang.contains(&lang.as_str()) {
        word = args.single_quoted::<String>()?;
    } else {
        lang = "en".to_string();
    }

    let url = format!(
        "https://api.dictionaryapi.dev/api/v2/entries/{}/{}",
        lang, word
    );
    let reqwest = ReqwestClient::new();
    let resp = reqwest
        .get(&url)
        .send()
        .await?
        .json::<Vec<DictionaryElement>>()
        .await;
    let definitions = if let Ok(x) = resp {
        x
    } else {
        msg.channel_id.say(ctx, "That word does not exist.").await?;
        return Ok(());
    };
    for definition in &definitions {
        msg.channel_id
            .send_message(ctx, |m| {
                m.embed(|embed| {
                    embed.title(capitalize_first(&definition.word));
                    if let Some(origin) = &definition.origin {
                        if origin != &"".to_string() {
                            embed.field("Origin:", &origin, true);
                        }
                    }

                    if let Some(phonetic) = &definition.phonetic {
                        if phonetic != &"".to_string() {
                            embed.field("Phonetic pronounciation:", &phonetic, true);
                        }
                    }
                    let mut text_definitions = String::new();
                    for meaning in &definition.meanings {
                        if let Some(pos) = &meaning.part_of_speech {
                            if pos != &"".to_string() {
                                text_definitions +=
                                    &format!("\n\n**{}**:\n", capitalize_first(&pos));
                            } else {
                                text_definitions += "\n\n**Unknown**:\n"
                            }
                        } else {
                            text_definitions += "\n\n**Unknown**:\n"
                        }

                        for definition in &meaning.definitions {
                            text_definitions += "\n**---**\n";
                            text_definitions += "- Definition:\n";
                            text_definitions += &definition.definition;
                            if let Some(example) = &definition.example {
                                if example != &"".to_string() {
                                    text_definitions += "\n- Example:\n";
                                    text_definitions += &example;
                                }
                            }
                        }
                    }
                    embed.description(&text_definitions)
                })
            })
            .await?;
    }
    Ok(())
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
#[command]
#[min_args(1)]
async fn poll(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let unformatted_time = args.single_quoted::<String>()?;
    let title = args.single_quoted::<String>()?;
    let unformatted_options = args.rest();
    let seconds = string_to_seconds(unformatted_time.clone());
    if seconds < 10 {
        msg.reply(ctx, "Duration is too short, stay within 10 sec to 2 mins")
            .await?;
        return Ok(());
    }
    if seconds > 120 {
        msg.reply(ctx, "Duration is too high, stay within 10 sec to 2 mins")
            .await?;
        return Ok(());
    }
    if unformatted_options.is_empty() {
        msg.reply(ctx, "Poll options are not provided").await?;
        return Ok(());
    }
    let options = unformatted_options.split(",").collect::<Vec<&str>>();
    if options.len() < 2 {
        msg.reply(ctx, "Requires alteast 2 options").await?;
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
    let poll_msg = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title(capitalize_first(&title));
                let mut text_definitions = String::new();
                for p in polls.iter() {
                    text_definitions +=
                        &format!("{} - **{}**\n", p.emoji, capitalize_first(&p.option));
                }
                e.description(&text_definitions);
                e.footer(|f| {
                    f.text(format!(
                        "Vote by reacting to the emojis, you have {} to vote",
                        unformatted_time
                    ))
                })
            })
        })
        .await?;

    for p in polls.iter() {
        poll_msg.react(ctx, p.emoji).await?;
    }
    let http = &ctx.http;
    let channel_id = poll_msg.channel_id.0 as u64;
    let message_id = poll_msg.id.0 as u64;
    let mut user_reactions: HashMap<u64, serenity::model::channel::ReactionType> = HashMap::new();
    let mut collector = poll_msg
        .await_reactions(&ctx)
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
    let mut new_poll_msg = ctx.http.get_message(channel_id, message_id).await?;
    let mut total_votes = 0;
    for p in polls.iter_mut() {
        for mr in &new_poll_msg.reactions {
            if mr.reaction_type.as_data() == p.emoji.to_string() {
                p.votes = mr.count - 1;
                total_votes += p.votes
            }
        }
    }
    new_poll_msg.delete_reactions(ctx).await?;
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
        .edit(ctx, |m| {
            m.embed(|embed| {
                embed.title(capitalize_first(&title));
                embed.description(&text_definitions)
            })
        })
        .await?;
    Ok(())
}

/// Get a random Chuck Norris joke.
#[command]
async fn chuck(ctx: &Context, msg: &Message) -> CommandResult {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.chucknorris.io/jokes/random")
        .send()
        .await?
        .json::<ChuckResponse>()
        .await?;
    msg.channel_id
        .send_message(ctx, |m| {
            m.content(
                resp.value
                    .unwrap_or("Chuck's a little busy here, try again later!".to_string()),
            )
        })
        .await?;
    Ok(())
}

/// Throw a dice.
#[command]
async fn dice(ctx: &Context, msg: &Message) -> CommandResult {
    let n: i64 = thread_rng().gen_range(1, 7);
    msg.reply(ctx, format!("You rolled a {}", n)).await?;
    Ok(())
}

/// Uwufy a text.
///
/// Usage:
/// `uwufy hello world`
#[command]
#[aliases(owofy, weebify, furryfy)]
#[min_args(1)]
async fn uwufy(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let message = args.message();
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
    msg.reply(ctx, &m).await?;
    Ok(())
}

/// Get a random fact.
#[command]
async fn fact(ctx: &Context, msg: &Message) -> CommandResult {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://nekos.life/api/v2/fact")
        .send()
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    msg.channel_id
        .send_message(ctx, |m| {
            m.content(
                resp.get("fact")
                    .unwrap_or(&"Couldn't find a fact, try again later!".to_string()),
            )
        })
        .await?;
    Ok(())
}

/// Why?.
#[command]
async fn why(ctx: &Context, msg: &Message) -> CommandResult {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://nekos.life/api/v2/why")
        .send()
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    msg.channel_id
        .send_message(ctx, |m| {
            m.content(resp.get("why").unwrap_or(&"Why".to_string()))
        })
        .await?;
    Ok(())
}

/// Eightball.
#[command]
async fn eightball(ctx: &Context, msg: &Message) -> CommandResult {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://nekos.life/api/v2/8ball")
        .send()
        .await?
        .json::<HashMap<String, String>>()
        .await?;
    if let Some(response) = resp.get("response") {
        if let Some(url) = resp.get("url") {
            msg.channel_id
                .send_message(ctx, |m| {
                    m.embed(|e| {
                        e.title("Eightball");
                        e.description(response);
                        e.image(url)
                    })
                })
                .await?;
            return Ok(());
        }
    }
    msg.channel_id
        .send_message(ctx, |m| m.content("Lost the eightball, try again later!"))
        .await?;
    Ok(())
}

/// Get all custom commands in this guild.
#[command]
async fn custom_commands(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap().0 as i64;
    let data = ctx.data.read().await;
    let client = data.get::<MongoClient>().unwrap();
    let cmds = Guild::from_db(client, guild_id).await?.custom_commands;
    let mut table = Table::new();
    table.force_no_tty().enforce_styling();
    table.set_content_arrangement(Dynamic).set_table_width(100);
    table.set_header(vec![
        Cell::new("Name").set_alignment(Center),
        Cell::new("Reply").set_alignment(Center),
    ]);
    for cmd in cmds.clone() {
        table.add_row(vec![
            Cell::new(cmd.name).set_alignment(Center),
            Cell::new(cmd.reply).set_alignment(Center),
        ]);
    }
    msg.channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title("Custom Commands");
                e.description(format!("```\n{}\n```", table))
            })
        })
        .await?;
    Ok(())
}

/// Get all self roles in this guild.
#[command]
async fn self_roles(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap().0 as i64;
    let data = ctx.data.read().await;
    let client = data.get::<MongoClient>().unwrap();
    let self_roles = Guild::from_db(client, guild_id).await?.self_roles;
    let mut table = Table::new();
    table.force_no_tty().enforce_styling();
    table.set_content_arrangement(Dynamic).set_table_width(100);
    table.set_header(vec![
        Cell::new("Role id").set_alignment(Center),
        Cell::new("Role name").set_alignment(Center),
    ]);
    let dguild = msg.guild(ctx).await.unwrap();
    for (role_id, role) in dguild.roles.clone() {
        let _role = role_id.0 as i64;
        if self_roles.contains(&_role) {
            table.add_row(vec![
                Cell::new(_role).set_alignment(Center),
                Cell::new(role.name).set_alignment(Center),
            ]);
        }
    }
    msg.channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title("Self roles");
                e.description(format!("```\n{}\n```", table))
            })
        })
        .await?;
    Ok(())
}

/// Get valorant server status.
#[command]
async fn valorant(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let supported_region = vec!["ap", "br", "eu", "kr", "latam", "na"];
    let mut region = "ap".to_string();
    if args.len() > 0 {
        let _r = args.single_quoted::<String>()?;
        if !supported_region.contains(&_r.as_str()) {
            region = _r;
        }
    }
    match get_status().await {
        Ok(status) => {
            let data = status.regions.iter().find(|&x| x.name == region).unwrap();
            if data.maintenances.is_empty() && data.incidents.is_empty() {
                msg.reply(ctx, "All fine! :thumbsup:").await?;
                return Ok(());
            }
            let mut fields = vec![("Region", region.as_str(), false)];
            data.maintenances
                .iter()
                .for_each(|item| fields.push(("Maintainance", item.description.as_str(), false)));
            data.maintenances
                .iter()
                .for_each(|item| fields.push(("Incident", item.description.as_str(), false)));
            msg.channel_id
                .send_message(ctx, |m| {
                    m.embed(|e| {
                        e.title("Valorant Server Staus");
                        e.fields(fields)
                    })
                })
                .await?;
        }
        Err(_) => {
            msg.reply(ctx, "Error getting valorant server status")
                .await?;
        }
    }
    Ok(())
}
