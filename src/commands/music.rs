use anyhow::Context;
use parse_duration::parse as parse_duration;
use poise::{
    command,
    serenity::model::{
        guild::Guild,
        id::{ChannelId, UserId},
        misc::Mentionable,
    },
};
use tokio::time::Instant;
use url::Url;

use crate::{
    constants::{
        DESCRIPTION_LENGTH_CUTOFF, LIVE_INDICATOR, MAX_LIST_ENTRY_LENGTH, MAX_SINGLE_ENTRY_LENGTH,
        UNKNOWN_TITLE,
    },
    data::{IdleGuildMap, LastMessageMap},
    types::{Error, PoiseContext},
    utils::{
        discord::{guild_check, reply, reply_embed},
        helpers::{chop_str, display_time_span, push_chopped_str},
    },
};

async fn join_internal<G, C>(
    ctx: &PoiseContext<'_>,
    guild_id: G,
    channel_id: C,
) -> Result<(), Error>
where
    G: Into<u64>,
    C: Into<u64>,
{
    let guild_id: u64 = guild_id.into();
    let (_, handler) = ctx
        .data()
        .songbird
        .join_gateway(guild_id, channel_id.into())
        .await;

    match handler {
        Ok(connection_info) => {
            match ctx
                .data()
                .lavalink
                .create_session_with_songbird(&connection_info)
                .await
            {
                Ok(_) => {
                    {
                        let data = ctx.discord().data.read().await;
                        let mut idle_hash_map =
                            data.get::<IdleGuildMap>().expect("msg").write().await;
                        idle_hash_map.insert(guild_id, Instant::now());
                    }

                    Ok(())
                }
                Err(e) => Err(Box::new(e)),
            }
        }
        Err(e) => Err(Box::new(e)),
    }
}

fn author_channel_id_from_guild(guild: &Guild, authour_id: &UserId) -> Option<ChannelId> {
    guild
        .voice_states
        .get(authour_id)
        .and_then(|voice_state| voice_state.channel_id)
}

/// Have bot join the voice channel you're in.
#[command(slash_command, aliases("j"))]
pub async fn join(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let guild = guild_check(ctx).await?;

    let channel_id = match author_channel_id_from_guild(&guild, &ctx.author().id) {
        Some(channel) => channel,
        None => {
            reply(ctx, "You must use this command while in a voice channel.").await?;
            return Ok(());
        }
    };

    match join_internal(&ctx, guild.id, channel_id).await {
        Ok(_) => reply(ctx, format!("Joined: {}", channel_id.mention())).await?,
        Err(e) => {
            reply(
                ctx,
                format!("Error joining {}: {}", channel_id.mention(), e),
            )
            .await?;
            return Ok(());
        }
    };

    Ok(())
}

/// Have bot leave the voice channel it's in, if any.
#[command(slash_command, aliases("l"))]
pub async fn leave(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let guild = guild_check(ctx).await?;

    let manager = &ctx.data().songbird;

    if manager.get(guild.id).is_some() {
        if let Err(e) = manager.remove(guild.id).await {
            reply(ctx, format!("Error leaving voice channel: {}", e)).await?;
        }

        let lava_client = &ctx.data().lavalink;
        lava_client.destroy(guild.id.0).await?;

        reply(ctx, "Left the voice channel.").await?;
    } else {
        reply(ctx, "Not in a voice channel.").await?;
    }

    Ok(())
}

/// Queue up a song or playlist from YouTube, Twitch, Vimeo, SoundCloud, etc.
#[command(slash_command, defer_response, aliases("p"))]
pub async fn play(
    ctx: PoiseContext<'_>,
    #[rest]
    #[description = "What to play."]
    query: String,
) -> Result<(), Error> {
    let guild = guild_check(ctx).await?;

    {
        let data = ctx.discord().data.read().await;
        let mut last_message_map = data.get::<LastMessageMap>().expect("msg").write().await;
        last_message_map.insert(guild.id.0, ctx.channel_id());
    }

    let manager = &ctx.data().songbird;
    let lava_client = &ctx.data().lavalink;

    if manager.get(guild.id).is_none() {
        let channel_id = match author_channel_id_from_guild(&guild, &ctx.author().id) {
            Some(channel) => channel,
            None => {
                reply(
                    ctx,
                    "You must use this command while either you or the bot is in a voice channel.",
                )
                .await?;
                return Ok(());
            }
        };

        if let Err(e) = join_internal(&ctx, guild.id, channel_id).await {
            reply(
                ctx,
                format!("Error joining {}: {}", channel_id.mention(), e),
            )
            .await?;
            return Ok(());
        }
    }

    let mut queueable_tracks = Vec::new();

    // Queue up any attachments
    match ctx {
        PoiseContext::Prefix(prefix_ctx) => {
            for attachment in &prefix_ctx.msg.attachments {
                // Verify the attachment is playable
                let playable_content = match &attachment.content_type {
                    Some(t) => t.starts_with("audio") || t.starts_with("video"),
                    None => false,
                };
                if !playable_content {
                    continue;
                }

                // Queue it up
                let mut query_result = lava_client.auto_search_tracks(&attachment.url).await?;
                for track in &mut query_result.tracks {
                    track.info = match &track.info {
                        Some(old_info) => {
                            let mut new_info = old_info.clone();
                            if old_info.title == UNKNOWN_TITLE {
                                new_info.title = attachment.filename.clone();
                            }
                            Some(new_info)
                        }
                        None => None,
                    }
                }
                queueable_tracks.extend_from_slice(&query_result.tracks)
            }
        }
        PoiseContext::Application(_) => {}
    }

    // Load the command query - if playable attachments were also with the message,
    // the attachments are queued first
    let query_information = lava_client.auto_search_tracks(&query).await?;

    let is_url = Url::parse(query.trim()).is_ok();

    // If the query was a URL, then it's likely a playlist where all retrieved
    // tracks are desired - otherwise, only queue the top result
    let query_tracks = if is_url {
        query_information.tracks.len()
    } else {
        1
    };

    queueable_tracks.extend_from_slice(
        &query_information
            .tracks
            .iter()
            .take(query_tracks)
            .cloned()
            .collect::<Vec<_>>(),
    );

    if queueable_tracks.is_empty() {
        reply(ctx, "Could not find anything for the search query.").await?;
        return Ok(());
    }

    let queueable_tracks_len = queueable_tracks.len();

    // For URLs that point to raw files, Lavalink seems to just return them with a
    // title of "Unknown title" - this is a slightly hacky solution to set the title
    // to the filename of the raw file
    if is_url && query_tracks == 1 {
        let track_info = &mut queueable_tracks[queueable_tracks_len - 1];
        if track_info.info.is_some() && track_info.info.as_ref().unwrap().title.eq(UNKNOWN_TITLE) {
            track_info.info = match &track_info.info {
                Some(old_info) => {
                    let mut new_info = old_info.clone();
                    new_info.title = Url::parse(old_info.uri.as_str())
                        .expect(
                            "Unable to parse track info URI when it should have been guaranteed \
							 to be valid",
                        )
                        .path_segments()
                        .expect("Unable to parse URI as a proper path")
                        .last()
                        .expect("Unable to find the last path segment of URI")
                        .to_owned();
                    Some(new_info)
                }
                None => None,
            };
        }
    }

    // Queue the tracks up
    for track in &queueable_tracks {
        if let Err(e) = lava_client
            .play(guild.id.0, track.clone())
            .requester(ctx.author().id.0)
            .queue()
            .await
        {
            reply(ctx, "Failed to queue up query result.").await?;
            eprintln!("Failed to queue up query result: {}", e);
            return Ok(());
        };
    }

    // Notify the user of the added tracks
    if queueable_tracks_len == 1 {
        let track_info = queueable_tracks[0].info.as_ref().unwrap();
        reply(
            ctx,
            format!(
                "Added to queue: [{}]({}) [{}]",
                chop_str(track_info.title.as_str(), MAX_SINGLE_ENTRY_LENGTH),
                track_info.uri,
                if track_info.is_stream {
                    LIVE_INDICATOR.to_owned()
                } else {
                    display_time_span(track_info.length)
                }
            ),
        )
        .await?;
    } else {
        let mut desc = String::from("Requested by ");
        desc.push_str(ctx.author().mention().to_string().as_str());
        desc.push('\n');
        for (i, track) in queueable_tracks.iter().enumerate() {
            let track_info = track.info.as_ref().unwrap();
            desc.push_str("- [");
            push_chopped_str(&mut desc, track_info.title.as_str(), MAX_LIST_ENTRY_LENGTH);
            desc.push_str("](");
            desc.push_str(track_info.uri.as_str());
            desc.push(')');
            if i < queueable_tracks_len - 1 {
                desc.push('\n');
                if desc.len() > DESCRIPTION_LENGTH_CUTOFF {
                    desc.push_str("*…the rest has been clipped*");
                    break;
                }
            }
        }
        reply_embed(ctx, |e| {
            e.title(format!("Added {} Tracks:", queueable_tracks_len))
                .description(desc)
        })
        .await?;
    }

    Ok(())
}

/// Skip the current track.
#[command(slash_command, aliases("next", "stop", "n", "s"))]
pub async fn skip(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let guild = guild_check(ctx).await?;

    let lava_client = &ctx.data().lavalink;

    if let Some(track) = lava_client.skip(guild.id.0).await {
        let track_info = track.track.info.as_ref().unwrap();
        // If the queue is now empty, the player needs to be stopped
        if lava_client
            .nodes()
            .await
            .get(&guild.id.0)
            .unwrap()
            .queue
            .is_empty()
        {
            lava_client
                .stop(guild.id.0)
                .await
                .with_context(|| "Failed to stop playback of the current track".to_owned())?;
        }
        reply(
            ctx,
            format!(
                "Skipped: [{}]({})",
                chop_str(track_info.title.as_str(), MAX_SINGLE_ENTRY_LENGTH),
                track_info.uri
            ),
        )
        .await?;
    } else {
        reply(ctx, "Nothing to skip.").await?;
    }

    Ok(())
}

/// Pause the current track.
///
/// The opposite of `resume`.
#[command(slash_command)]
pub async fn pause(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let guild = guild_check(ctx).await?;

    let lava_client = &ctx.data().lavalink;

    if let Err(e) = lava_client.pause(guild.id.0).await {
        reply(ctx, "Failed to pause playback.").await?;
        eprintln!("Failed to pause playback: {}", e);
        return Ok(());
    };

    reply(ctx, "Paused playback.").await?;

    Ok(())
}

/// Resume the current track.
///
/// The opposite of `pause`.
#[command(slash_command)]
pub async fn resume(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let guild = guild_check(ctx).await?;

    let lava_client = &ctx.data().lavalink;

    if let Err(e) = lava_client.resume(guild.id.0).await {
        reply(ctx, "Failed to resume playback.").await?;
        eprintln!("Failed to resume playback: {}", e);
        return Ok(());
    };

    reply(ctx, "Resumed playback.").await?;

    Ok(())
}

/// Seek to a specific time in the current track.
///
/// You can specify the time to skip to as a timecode (`2:35`) or as individual
/// time values (`2m35s`).
///
/// If the time specified is past the end of the track, the track ends.
#[command(slash_command, aliases("scrub", "jump"))]
pub async fn seek(
    ctx: PoiseContext<'_>,
    #[rest]
    #[description = "What time to skip to."]
    time: String,
) -> Result<(), Error> {
    // Constants
    const COLON: char = ':';
    const DECIMAL: char = '.';

    // Parse the time - this is a little hacky and gross, but it allows for support
    // of timecodes like `2:35`. This is more ergonomic for users than something
    // like `2m35s`, and this way both formats are supported.
    let mut invalid_value = false;
    let mut time_prepared = String::with_capacity(time.len());
    'prepare_time: for timecode in time.split_whitespace() {
        // First iteration to find indices and make sure the timecode is valid
        let mut colon_index_first = None;
        let mut colon_index_second = None;
        let mut decimal_index = None;
        for (i, c) in timecode.chars().enumerate() {
            if c == COLON {
                if colon_index_first.is_none() {
                    colon_index_first = Some(i);
                } else if colon_index_second.is_none() {
                    colon_index_second = Some(i);
                } else {
                    // Maximum of two colons in a timecode
                    invalid_value = true;
                    break 'prepare_time;
                }
                if decimal_index.is_some() {
                    // Colons don't come after decimals
                    invalid_value = true;
                    break 'prepare_time;
                }
            } else if c == DECIMAL {
                if decimal_index.is_none() {
                    decimal_index = Some(i);
                } else {
                    // Only one decimal value
                    invalid_value = true;
                    break 'prepare_time;
                }
            }
        }

        // Second iteration using those indices to convert the timecode to a duration
        // representation
        let mut new_word = String::with_capacity(timecode.len());
        for (i, c) in timecode.chars().enumerate() {
            if colon_index_first.is_some() && i == colon_index_first.unwrap() {
                if colon_index_second.is_some() {
                    new_word.push('h');
                } else {
                    new_word.push('m');
                }
            } else if colon_index_second.is_some() && i == colon_index_second.unwrap() {
                new_word.push('m');
            } else if decimal_index.is_some() && i == decimal_index.unwrap() {
                new_word.push('s');
            } else {
                new_word.push(c);
            }
        }
        if decimal_index.is_some() {
            new_word.push_str("ms");
        } else if colon_index_first.is_some() {
            new_word.push('s');
        }

        // Push the prepared timecode to the result
        time_prepared.push_str(new_word.as_str());
        time_prepared.push(' ');
    }
    if invalid_value {
        reply(ctx, "Invalid value for time.").await?;
        return Ok(());
    }

    let time_dur = match parse_duration(time_prepared.as_str()) {
        Ok(duration) => duration,
        Err(_) => {
            reply(ctx, "Invalid value for time.").await?;
            return Ok(());
        }
    };

    // Seek to the parsed time
    let guild = guild_check(ctx).await?;

    let lava_client = &ctx.data().lavalink;

    if let Err(e) = lava_client.seek(guild.id.0, time_dur).await {
        reply(ctx, "Failed to seek to the specified time.").await?;
        eprintln!("Failed to seek to the specified time: {}", e);
        return Ok(());
    };

    reply(ctx, "Scrubbed to the specified time.").await?;

    Ok(())
}

/// Clear the playback queue.
///
/// In addition to clearing the queue, this also resets the queue position for
/// new tracks. This is the only way this happens other than when the bot goes
/// offline.
#[command(slash_command, aliases("c"))]
pub async fn clear(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let guild = guild_check(ctx).await?;

    let lava_client = &ctx.data().lavalink;

    while lava_client.skip(guild.id.0).await.is_some() {}
    lava_client
        .stop(guild.id.0)
        .await
        .with_context(|| "Failed to stop playback of the current track".to_owned())?;
    reply(ctx, "The queue is now empty.").await?;

    Ok(())
}

/// Show what's currently playing, and how far in you are in the track.
///
/// If the track has a defined end point, a progress bar will be displayed.
/// Otherwise, if the track is a live stream, only the time it's been playing
/// will be displayed.
#[command(
    slash_command,
    rename = "nowplaying",
    aliases("np", "position", "current", "rn")
)]
pub async fn now_playing(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let guild = guild_check(ctx).await?;

    let lava_client = &ctx.data().lavalink;

    let mut something_playing = false;
    if let Some(node) = lava_client.nodes().await.get(&guild.id.0) {
        if let Some(now_playing) = &node.now_playing {
            let track_info = now_playing.track.info.as_ref().unwrap();
            reply_embed(ctx, |e| {
                e.title("Now Playing")
                    .field(
                        "Track:",
                        format!(
                            "[{}]({})",
                            chop_str(track_info.title.as_str(), MAX_SINGLE_ENTRY_LENGTH),
                            track_info.uri,
                        ),
                        false,
                    )
                    .field("Duration:", display_time_span(track_info.length), true)
                    .field(
                        "Requested By:",
                        UserId(
                            now_playing
                                .requester
                                .expect("Expected a requester associated with a playing track")
                                .0,
                        )
                        .mention(),
                        true,
                    )
            })
            .await?;
            something_playing = true;
        }
    }
    if !something_playing {
        reply(ctx, "Nothing is playing at the moment.").await?;
    }

    Ok(())
}

/// Show the playback queue.
#[command(slash_command, aliases("q"))]
pub async fn queue(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let guild = guild_check(ctx).await?;

    let lava_client = &ctx.data().lavalink;

    let mut something_in_queue = false;
    if let Some(node) = lava_client.nodes().await.get(&guild.id.0) {
        let queue = &node.queue;
        let queue_len = queue.len();

        if queue_len > 0 {
            something_in_queue = true;

            let mut desc = String::new();
            for (i, queued_track) in queue.iter().enumerate() {
                let track_info = queued_track.track.info.as_ref().unwrap();
                desc.push_str(format!("`{}.` [", i + 1).as_str());
                push_chopped_str(&mut desc, track_info.title.as_str(), MAX_LIST_ENTRY_LENGTH);
                desc.push_str("](");
                desc.push_str(track_info.uri.as_str());
                desc.push(')');
                if i < queue_len - 1 {
                    desc.push('\n');
                    if desc.len() > DESCRIPTION_LENGTH_CUTOFF {
                        desc.push_str("*…the rest has been clipped*");
                        break;
                    }
                }
            }
            reply_embed(ctx, |e| {
                e.title(if queue_len != 1 {
                    format!("Queue ({} total tracks):", queue_len)
                } else {
                    format!("Queue ({} total track):", queue_len)
                })
                .description(desc)
            })
            .await?;
        }
    }
    if !something_in_queue {
        reply(ctx, "Nothing is in the queue.").await?;
    }

    Ok(())
}
