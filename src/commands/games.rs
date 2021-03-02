use crate::utils::apis::*;
use comfy_table::{Cell, CellAlignment::Center, ContentArrangement::Dynamic, Table};
use html_escape;
use serenity::{
    builder::CreateMessage,
    collector::MessageCollectorBuilder,
    framework::standard::{macros::command, Args, CommandResult},
    futures::stream::StreamExt,
    model::{channel::Message, id::UserId, misc::Mentionable},
    prelude::Context,
};
use std::{collections::HashMap, time::Duration};
use strsim::normalized_levenshtein;

fn _trivia_msg<S: Into<String>>(content: S, footer: Option<S>) -> CreateMessage<'static> {
    let mut m = CreateMessage::default();
    m.embed(|e| {
        e.description(content.into());
        if let Some(f_text) = footer {
            e.footer(|f| f.text(f_text.into()));
        }
        e
    });
    m
}

/// Trivia competition.
///
/// Categories available:
///     Any
///     GeneralKnowledge
///     EntertainmentBooks
///     EntertainmentFilm
///     EntertainmentMusic
///     EntertainmentMusicalsAndTheatres
///     EntertainmentTelevision
///     EntertainmentVideoGames
///     EntertainmentBoardGames
///     ScienceNature
///     ScienceComputers
///     ScienceMathematics
///     Mythology
///     Sports
///     Geography
///     History
///     Politics
///     Art
///     Celebrities
///     Animals
///     Vehicles
///     EntertainmentComics
///     ScienceGadgets
///     EntertainmentJapaneseAnimeAndManga
///     EntertainmentCartoonAndAnimations
///
/// Difficulty
///     Any
///     Easy (default)
///     Medium
///     Hard
///
/// Usage: `trivia <amount>` or `trivia <amount> <category> <difficulty>`
#[command]
#[aliases(quiz)]
#[min_args(1)]
async fn trivia(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let amount = args.single::<usize>().unwrap();
    let category = args
        .single::<TriviaCategory>()
        .unwrap_or(TriviaCategory::Any);
    let difficulty = args
        .single::<TriviaDifficulty>()
        .unwrap_or(TriviaDifficulty::Easy);
    let rs = get_trivia(amount, category, difficulty).await?;
    if rs.response_code != 0 {
        msg.channel_id
            .say(&ctx, "Error getting the questions, please try again")
            .await?;
        return Ok(());
    }
    let quiz_channel = msg
        .guild(&ctx)
        .await
        .unwrap()
        .create_channel(&ctx, |c| {
            c.name("Temporary Quiz Channel")
                .topic("Welcome to Temporary Quiz Channel, quiz will begin shortly")
        })
        .await?;
    let mut host = msg
        .channel_id
        .send_message(&ctx, |m| {
            m.clone_from(&_trivia_msg(
                "Quiz about to start in 5 seconds, move to **Temporary Quiz Channel** to participate",
                Some(&format!("Amount: {} Difficulty: {:?} Category: {:?}", amount, difficulty, category )),
            ));
            m
        })
        .await?;
    tokio::time::sleep(Duration::from_secs(5)).await;
    let mut scores: HashMap<UserId, usize> = HashMap::new();
    for clue in rs.results {
        let question = html_escape::decode_html_entities(&clue.question).to_string();
        let answer = html_escape::decode_html_entities(&clue.correct_answer).to_string();
        let _m = &mut _trivia_msg(
            question,
            Some(String::from("You got 15 seconds to answer this question.")),
        );
        quiz_channel.send_message(&ctx, |_| _m).await?;
        let mut collector = MessageCollectorBuilder::new(&ctx)
            .channel_id(quiz_channel.id)
            .filter(|m| !m.author.bot)
            .timeout(Duration::from_secs(15))
            .await;
        let mut answered = false;
        while let Some(m) = collector.next().await {
            if !scores.contains_key(&m.author.id) {
                scores.insert(m.author.id, 0);
            }
            let distance =
                normalized_levenshtein(&m.content.to_lowercase(), &answer.to_lowercase());
            if distance > 0.9 {
                collector.stop();
                let _ = m.react(ctx, '✔').await;
                let _s = scores.get_mut(&m.author.id).unwrap();
                *_s += 1;
                quiz_channel
                    .send_message(&ctx, |x| {
                        x.clone_from(&_trivia_msg(
                            &format!(
                                "{} got the answer correct **+1**, moving onto next question!",
                                m.author.mention()
                            ),
                            Some(&format!("Answer was: {}", answer)),
                        ));
                        x
                    })
                    .await?;
                answered = true;
                break;
            } else if distance > 0.8 {
                let _ = m.react(ctx, '🤏').await;
            } else {
                let _ = m.react(ctx, '❌').await;
            }
        }
        if !answered {
            quiz_channel
                .send_message(&ctx, |m| {
                    m.clone_from(&_trivia_msg(
                        "Nobody got the answer correct, moving onto next question!",
                        Some(&format!("Answer was: {}", answer)),
                    ));
                    m
                })
                .await?;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    quiz_channel
                .send_message(&ctx, |m| {
                    m.clone_from(&_trivia_msg(
                        "Results will be posted in the main channel, this channel will be automatically deleted in 3 seconds",None
                    ));
                    m
                })
                .await?;
    tokio::time::sleep(Duration::from_secs(3)).await;
    quiz_channel.delete(&ctx).await?;
    let mut table = Table::new();
    table.force_no_tty().enforce_styling();
    table.set_content_arrangement(Dynamic).set_table_width(100);
    table.set_header(vec![
        Cell::new("Participant").set_alignment(Center),
        Cell::new("Score").set_alignment(Center),
    ]);
    for (user_id, score) in scores.iter() {
        let _member = ctx
            .cache
            .member(msg.guild_id.unwrap(), user_id)
            .await
            .unwrap_or(
                ctx.http
                    .get_member(msg.guild_id.unwrap().0 as u64, user_id.0 as u64)
                    .await?,
            );
        table.add_row(vec![
            Cell::new(_member.display_name()).set_alignment(Center),
            Cell::new(score.to_string()).set_alignment(Center),
        ]);
    }
    host.edit(ctx, |m| {
        m.embed(|e| {
            e.title("Trivia Scoreboard");
            e.description(format!("```\n{}\n```", table))
        })
    })
    .await?;

    Ok(())
}
