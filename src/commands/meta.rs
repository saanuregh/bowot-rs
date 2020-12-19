use crate::utils::basic_functions::{
    get_meta_info, get_rest_latency, get_shard_latency, MetaInfoResult,
};
use serenity::{
    framework::standard::{macros::command, CommandResult},
    model::{channel::Message, Permissions},
    prelude::Context,
};

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
        m.reference_message(msg);
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
    let shard_latency = get_shard_latency(ctx).await;
    let (rest_latency, mut message) = get_rest_latency(ctx, msg.channel_id.0).await?;
    message
        .edit(ctx, |m| {
            m.content("");
            m.embed(|e| {
                e.title("Ping").fields(vec![
                    ("Gateway", shard_latency + "ms", false),
                    ("REST", rest_latency.to_string() + "ms", false),
                ])
            })
        })
        .await?;
    Ok(())
}

/// Sends information about the bot.
#[command]
#[aliases(info)]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    let (rest_latency, mut message) = get_rest_latency(ctx, msg.channel_id.0).await?;
    let MetaInfoResult {
        shard_latency,
        uptime,
        memory_usage,
        cpu_usage,
        version,
        hoster_tag,
        hoster_id,
        bot_name,
        bot_icon,
        num_guilds,
        num_shards,
        num_channels,
        num_priv_channels,
    } = get_meta_info(ctx).await;
    message.edit(ctx, |m| {
        m.content("");
        m.embed(|e| {
            e.title(format!("**{}** - v{}", bot_name, version));
            e.url("https://github.com/saanuregh/bowot-rs");
            e.description("General Purpose Discord Bot made in [Rust](https://www.rust-lang.org/) using [serenity.rs](https://github.com/serenity-rs/serenity)\nHaving any issues, just dm me 😊.");
            e.field("Statistics:", format!("Shards: {}\nGuilds: {}\nChannels: {}\nPrivate Channels: {}", num_shards, num_guilds, num_channels, num_priv_channels), true);
            e.field("Currently hosted by:", format!("Tag: {}\nID: {}", hoster_tag, hoster_id), true);
            e.field("Latency:", format!("Gateway:\n`{}ms`\nREST:\n`{}ms`", shard_latency, rest_latency), true);
            e.field("CPU Usage:", format!("`{:.2} %`",cpu_usage), true);
            e.field("Memory Usage:", format!("`{} KB`", memory_usage), true);
            e.field("Uptime:", format!("`{}`", uptime), true);
            e.thumbnail(bot_icon);
            e
        });
        m
    }).await?;
    Ok(())
}
