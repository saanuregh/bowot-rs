use crate::{ShardManagerContainer, Uptime};
use num_format::{Locale, ToFormattedString};
use serde_json::json;
use serenity::{
    client::bridge::gateway::ShardId,
    framework::standard::{macros::command, CommandResult},
    model::{channel::Message, Permissions},
    prelude::Context,
};
use std::{
    fs::{read_to_string, File},
    io::prelude::*,
    process::id,
    time::Instant,
};
use tokio::process::Command;
use toml::Value;
use walkdir::WalkDir;

/// This command just sends an invite of the bot with the required permissions.
#[command]
async fn invite(ctx: &Context, msg: &Message) -> CommandResult {
    let _p = vec![
        Permissions::MANAGE_GUILD,
        Permissions::MANAGE_ROLES,
        Permissions::MANAGE_CHANNELS,
        Permissions::KICK_MEMBERS,
        Permissions::BAN_MEMBERS,
        Permissions::CREATE_INVITE,
        Permissions::MANAGE_WEBHOOKS,
        Permissions::READ_MESSAGES,
        Permissions::SEND_MESSAGES,
        Permissions::MANAGE_MESSAGES,
        Permissions::EMBED_LINKS,
        Permissions::ATTACH_FILES,
        Permissions::READ_MESSAGE_HISTORY,
        Permissions::USE_EXTERNAL_EMOJIS,
        Permissions::ADD_REACTIONS,
        Permissions::SPEAK,
        Permissions::CONNECT,
    ];
    let mut permissions = Permissions::empty();
    _p.iter().for_each(|&p| permissions.set(p, true));
    let url = ctx
        .cache
        .current_user()
        .await
        .invite_url(ctx, permissions)
        .await?;
    msg.channel_id.send_message(ctx, |m| {
        m.embed( |e| {
            e.title("Invite Link");
            e.url(url);
            e.description("__**Reason for each permission**__");
            e.fields(vec![
                ("Manage Guild", "Be able to manage server.", true),
                ("Manage Roles", "Be able to manage roles of server and members.", true),
                ("Manage Channels", "Be able to mute members on the channel without having to create a role for it.", true),
                ("Kick Members", "Kick/GhostBan moderation command.", true),
                ("Ban Members", "Ban moderation command.", true),
                ("Create Invite", "Allow creation of rich invite.", true),
                ("Manage Webhooks", "For all the commands that can be ran on a schedule, so it's more efficient.", true),
                ("Read Messages", "So the bot can read the messages to know when a command was invoked and such.", true),
                ("Send Messages", "So the bot can send the messages it needs to send.", true),
                ("Manage Messages", "Be able to manage messages, like for clear command.", true),
                ("Embed Links", "For the tags to be able to embed images.", true),
                ("Attach Files", "For the tags to be able to attach files.", true),
                ("Read Message History", "This is a required permission for every paginated command.", true),
                ("Use External Emojis", "For all the commands that use emojis for better emphasis.", true),
                ("Add Reactions", "To be able to add reactions for all the paginated commands.", true),
                ("Speak", "To be able to play music on that voice channel.", true),
                ("Connect", "To be able to connect to a voice channel.", true),
       ]);
            e
        });

        m
    }).await?;
    Ok(())
}

// Sends the latency of the bot to the shards.
#[command]
#[aliases("pong", "latency")]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(v) => v,
        None => {
            msg.reply(ctx, "There was a problem getting the shard manager")
                .await?;
            return Ok(());
        }
    };
    let manager = shard_manager.lock().await;
    let runners = manager.runners.lock().await;
    let runner = match runners.get(&ShardId(ctx.shard_id)) {
        Some(runner) => runner,
        None => {
            msg.reply(ctx, "No shard found").await?;
            return Ok(());
        }
    };
    let shard_latency = match runner.latency {
        Some(ms) => format!("{:.2}ms", ms.as_micros() as f32 / 1000.0),
        _ => String::new(),
    };
    let map = json!({"content" : "Calculating latency..."});
    let now = Instant::now();
    let mut message = ctx.http.send_message(msg.channel_id.0, &map).await?;
    let rest_latency = now.elapsed().as_millis();
    message
        .edit(ctx, |m| {
            m.content(format!(
                "Ping?\nGateway: {}\nREST: {}ms",
                shard_latency, rest_latency
            ))
        })
        .await?;
    Ok(())
}

/// Sends information about the bot.
#[command]
#[aliases(info)]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    let shard_latency = {
        let data = ctx.data.read().await;
        let shard_manager = data.get::<ShardManagerContainer>().unwrap();

        let manager = shard_manager.lock().await;
        let runners = manager.runners.lock().await;

        let runner_raw = runners.get(&ShardId(ctx.shard_id));
        if let Some(runner) = runner_raw {
            match runner.latency {
                Some(ms) => format!("{}ms", ms.as_millis()),
                _ => "?ms".to_string(),
            }
        } else {
            "?ms".to_string()
        }
    };
    let uptime = {
        let data = ctx.data.read().await;
        let instant = data.get::<Uptime>().unwrap();
        let duration = instant.elapsed();
        humantime::format_duration(duration)
    };
    let map = json!({"content" : "Calculating latency..."});
    let now = Instant::now();
    let mut message = ctx.http.send_message(msg.channel_id.0, &map).await?;
    let rest_latency = now.elapsed().as_millis();
    let pid = id().to_string();
    let full_stdout = Command::new("sh")
        .arg("-c")
        .arg(format!(r"pmap {} | tail -n 1 | awk '/[0-9]K/{{print $2}}'", &pid).as_str())
        .output()
        .await
        .expect("failed to execute process");
    let reasonable_stdout = Command::new("sh")
        .arg("-c")
        .arg(
            format!(
                "pmap {} | head -n 2 | tail -n 1 | awk '/[0-9]K/{{print $2}}'",
                &pid
            )
            .as_str(),
        )
        .output()
        .await
        .expect("failed to execute process");

    let mut full_mem = String::from_utf8(full_stdout.stdout).unwrap();
    let mut reasonable_mem = String::from_utf8(reasonable_stdout.stdout).unwrap();
    full_mem.pop();
    full_mem.pop();
    reasonable_mem.pop();
    reasonable_mem.pop();
    let version = {
        let mut file = File::open("Cargo.toml")?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let data = contents.parse::<Value>().unwrap();
        let version = data["package"]["version"].as_str().unwrap();
        version.to_string()
    };
    let (hoster_tag, hoster_id) = {
        let app_info = ctx.http.get_current_application_info().await?;
        (app_info.owner.tag(), app_info.owner.id)
    };
    let current_user = ctx.cache.current_user().await;
    let bot_name = &current_user.name;
    let bot_icon = &current_user.avatar_url();
    let num_guilds = ctx.cache.guilds().await.len();
    let num_shards = ctx.cache.shard_count().await;
    let num_channels = ctx.cache.guild_channel_count().await;
    let num_priv_channels = ctx.cache.private_channels().await.len();
    let mut c_blank = 0;
    let mut c_comment = 0;
    let mut c_code = 0;
    let mut c_lines = 0;
    let mut command_count = 0;
    for entry in WalkDir::new("src") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_file() {
            let count = loc::count(path.to_str().unwrap());
            let text = read_to_string(&path)?;
            command_count += text.match_indices("#[command]").count();
            c_blank += count.blank;
            c_comment += count.comment;
            c_code += count.code;
            c_lines += count.lines;
        }
    }
    message.edit(ctx, |m| {
        m.content("");
        m.embed(|e| {
            e.title(format!("**{}** - v{}", bot_name, version));
            e.url("https://github.com/saanuregh/bowot-rs");
            e.description("General Purpose Discord Bot made in [Rust](https://www.rust-lang.org/) using [serenity.rs](https://github.com/serenity-rs/serenity)\nHaving any issues, just dm me 😊.");
            e.field("Statistics:", format!("Shards: {}\nGuilds: {}\nChannels: {}\nPrivate Channels: {}", num_shards, num_guilds, num_channels, num_priv_channels), true);
            e.field("Lines of code:", format!("Blank: {}\nComment: {}\nCode: {}\nTotal Lines: {}", c_blank, c_comment, c_code, c_lines), true);
            e.field("Currently hosted by:", format!("Tag: {}\nID: {}", hoster_tag, hoster_id), true);
            e.field("Latency:", format!("Gateway:\n`{}`\nREST:\n`{}ms`", shard_latency, rest_latency), true);
            e.field("Memory usage:", format!("Complete:\n`{} KB`\nBase:\n`{} KB`",
                                            &full_mem.parse::<u32>().expect("NaN").to_formatted_string(&Locale::en),
                                            &reasonable_mem.parse::<u32>().expect("NaN").to_formatted_string(&Locale::en)
                                            ), true);
            e.field("Somewhat Static Stats:", format!("Command Count:\n`{}`\nUptime:\n`{}`", command_count, uptime), true);
            if let Some(x) = bot_icon {
                e.thumbnail(x);
            }
            e
        });
        m
    }).await?;
    Ok(())
}
