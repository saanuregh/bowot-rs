use poise::serenity_prelude::{OAuth2Scope, Permissions};

use crate::{
    types::{Error, PoiseContext},
    utils::discord::{get_meta_info, get_rest_latency, reply_embed, MetaInfoResult},
};

/// Register application commands in this guild or globally
///
/// Run with no arguments to register in guild, run with argument "global" to
/// register globally.
#[poise::command(prefix_command, owners_only, hide_in_help)]
pub async fn register(ctx: PoiseContext<'_>, #[flag] global: bool) -> Result<(), Error> {
    poise::samples::register_application_commands(ctx.into(), global).await?;

    Ok(())
}

/// Show help menu
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: PoiseContext<'_>,
    #[description = "A specific command to show help about."] command: Option<String>,
) -> Result<(), Error> {
    poise::samples::help(
        ctx,
        command.as_deref(),
        "",
        poise::samples::HelpResponseMode::Ephemeral,
    )
    .await?;

    Ok(())
}

/// Sends information about the bot.
#[poise::command(slash_command)]
pub async fn about(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let discord_ctx = ctx.discord();
    let channel_id = ctx.channel_id();
    let rest_latency = get_rest_latency(discord_ctx, channel_id).await?;
    let MetaInfoResult {
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
    } = get_meta_info(discord_ctx).await;
    reply_embed(ctx, |e| {
		e.title(format!("**{}** - v{}", bot_name, version));
		e.url("https://github.com/saanuregh/bowot-rs");
		e.description("General Purpose Discord Bot made in [Rust](https://www.rust-lang.org/) using [serenity.rs](https://github.com/serenity-rs/serenity)\nHaving any issues, just dm me 😊.");
		e.field("Statistics:", format!("Shards: {}\nGuilds: {}\nChannels: {}\nPrivate Channels: {}", num_shards, num_guilds, num_channels, num_priv_channels), true);
		e.field("Currently hosted by:", format!("Tag: {}\nID: {}", hoster_tag, hoster_id), true);
		e.field("Latency:", format!("REST:\n`{}ms`", rest_latency), true);
		e.field("CPU Usage:", format!("`{:.2} %`",cpu_usage), true);
		e.field("Memory Usage:", format!("`{} KB`", memory_usage), true);
		e.field("Uptime:", format!("`{}`", uptime), true);
		e.thumbnail(bot_icon);
		e
	}).await?;

    Ok(())
}

/// Sends the latency of the bot to the shards.
#[poise::command(slash_command)]
pub async fn ping(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let discord_ctx = ctx.discord();
    let channel_id = ctx.channel_id();
    let rest_latency = get_rest_latency(discord_ctx, channel_id).await?;
    reply_embed(ctx, |e| {
        e.title("Ping");
        e.field("Latency:", format!("REST:\n`{}ms`", rest_latency), true);
        e
    })
    .await?;

    Ok(())
}

/// This command just sends an invite of the bot with the required permissions.
#[poise::command(slash_command)]
pub async fn invite(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let discord_ctx = ctx.discord();

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
        Permissions::USE_PRIVATE_THREADS,
        Permissions::USE_PUBLIC_THREADS,
        Permissions::USE_SLASH_COMMANDS,
    ];
    let scopes = vec![OAuth2Scope::Bot, OAuth2Scope::ApplicationsCommands];

    let mut permissions = Permissions::empty();
    _p.iter().for_each(|&p| permissions.set(p, true));
    let url = discord_ctx
        .cache
        .current_user()
        .invite_url_with_oauth2_scopes(discord_ctx, permissions, &scopes)
        .await?;

    reply_embed(ctx, |e| {
        e.title("Invite Link");
        e.url(url);
        e.description("__**Reason for each permission**__");
        e.fields(vec![
            ("Manage Guild", "Be able to manage server.", true),
            (
                "Manage Roles",
                "Be able to manage roles of server and members.",
                true,
            ),
            (
                "Manage Channels",
                "Be able to mute members on the channel without having to create a role for it.",
                true,
            ),
            ("Kick Members", "Kick/GhostBan moderation command.", true),
            ("Ban Members", "Ban moderation command.", true),
            ("Create Invite", "Allow creation of rich invite.", true),
            (
                "Manage Webhooks",
                "For all the commands that can be ran on a schedule, so it's more efficient.",
                true,
            ),
            (
                "Read Messages",
                "So the bot can read the messages to know when a command was invoked and such.",
                true,
            ),
            (
                "Send Messages",
                "So the bot can send the messages it needs to send.",
                true,
            ),
            (
                "Manage Messages",
                "Be able to manage messages, like for clear command.",
                true,
            ),
            (
                "Embed Links",
                "For the tags to be able to embed images.",
                true,
            ),
            (
                "Attach Files",
                "For the tags to be able to attach files.",
                true,
            ),
            (
                "Read Message History",
                "This is a required permission for every paginated command.",
                true,
            ),
            (
                "Use External Emojis",
                "For all the commands that use emojis for better emphasis.",
                true,
            ),
            (
                "Add Reactions",
                "To be able to add reactions for all the paginated commands.",
                true,
            ),
            (
                "Speak",
                "To be able to play music on that voice channel.",
                true,
            ),
            ("Connect", "To be able to connect to a voice channel.", true),
        ]);
        e
    })
    .await?;

    Ok(())
}
