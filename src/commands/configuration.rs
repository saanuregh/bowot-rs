use crate::{
    data::{GuildCacheStore, PoolContainer},
    database::Guild,
    framework::MASTER_GROUP,
};
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
    prelude::Context,
};

/// Configures the bot for the guild/server it was invoked on.
///
/// Configurable aspects:
/// `prefix`: Changes the bot prefix.
/// `add_trigger_phrase`: Add a trigger phrase.
/// `remove_trigger_phrase`: Remove a trigger phrase.
/// `disable_command`: Disables a command.
/// `enable_command`: Enables a disabled command.
#[command]
#[required_permissions(MANAGE_GUILD)]
#[only_in("guilds")]
#[aliases(server)]
#[sub_commands(
    prefix,
    add_trigger_phrase,
    remove_trigger_phrase,
    disable_command,
    enable_command
)]
async fn guild(_ctx: &Context, _msg: &Message, _args: Args) -> CommandResult {
    Ok(())
}

async fn _update_guild_cache_store(ctx: &Context, guild_id: impl Into<i64>) -> anyhow::Result<()> {
    let data = ctx.data.read().await;
    let db = data
        .get::<PoolContainer>()
        .expect("Expected DBPool to be in TypeMap");
    let guild_cache_store = data
        .get::<GuildCacheStore>()
        .expect("Expected GuildCacheStore to be in TypeMap");
    guild_cache_store.update(db, guild_id).await;

    Ok(())
}

/// Change the command prefix on this guild.
///
/// Usage: `config guild prefix !`
#[command]
#[min_args(1)]
async fn prefix(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let prefix = args.message();
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    let db = data.get::<PoolContainer>().unwrap();
    Guild::new(db, guild_id).set_prefix(prefix).await?;
    _update_guild_cache_store(ctx, guild_id).await?;
    msg.reply(
        ctx,
        format!("Successfully changed your prefix to `{}`", prefix),
    )
    .await?;

    Ok(())
}

async fn _cr_trigger_phrase(
    ctx: &Context,
    msg: &Message,
    mut args: Args,
    create: bool,
) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let phrase = args.single::<String>()?;
    let reply = args.rest().to_string();
    let data = ctx.data.read().await;
    let db = data.get::<PoolContainer>().unwrap();
    let db_guild = Guild::new(db, guild_id);
    let content = if create {
        db_guild.insert_trigger(phrase.clone(), reply).await?;
        format!("Successfully added trigger phrase `{}`", phrase)
    } else {
        db_guild.delete_trigger(phrase.clone()).await?;
        format!("Successfully removed trigger phrase `{}`", phrase)
    };
    _update_guild_cache_store(ctx, guild_id).await?;
    msg.reply(ctx, content).await?;

    Ok(())
}

/// Add a trigger phrase on this guild.
///
/// Usage: `config guild add_trigger_phrase <trigger> <reply>`
/// Usage: `config guild add_trigger_phrase hello world`
#[command]
#[min_args(2)]
async fn add_trigger_phrase(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    _cr_trigger_phrase(ctx, msg, args, true).await
}

/// Remove a trigger phrase on this guild.
///
/// Usage: `config guild remove_trigger_phrase <trigger>`
/// Usage: `config guild remove_trigger_phrase hello`
#[command]
#[min_args(1)]
async fn remove_trigger_phrase(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    _cr_trigger_phrase(ctx, msg, args, false).await
}

async fn _change_command_status_command(
    ctx: &Context,
    msg: &Message,
    mut args: Args,
    enable: bool,
) -> CommandResult {
    let command_name = args.single_quoted::<String>()?;
    let command_exist = MASTER_GROUP.options.sub_groups.iter().any(|group| {
        group
            .options
            .commands
            .iter()
            .any(|command| command.options.names.contains(&command_name.as_str()))
    });
    let mut content = format!("Command `{}` not found", command_name);
    if command_exist {
        let guild_id = msg.guild_id.unwrap();
        let data = ctx.data.read().await;
        let db = data.get::<PoolContainer>().unwrap();
        let db_guild = Guild::new(db, guild_id);
        let mut disabled_commands = db_guild.get_disabled_commands().await?;
        if enable {
            disabled_commands.retain(|x| *x != command_name);
            content = format!("Command `{}` successfully re-enabled", command_name);
        } else {
            if !disabled_commands.contains(&command_name) {
                disabled_commands.push(command_name.clone());
            }
            content = format!("Command `{}` successfully disabled", command_name);
        }
        db_guild.set_disabled_commands(disabled_commands).await?;
        _update_guild_cache_store(ctx, guild_id).await?;
    }
    msg.reply(ctx, content).await?;

    Ok(())
}

/// Disables a command on this guild.
///
/// Usage: `config guild disable_command urban`
#[command]
#[min_args(1)]
async fn disable_command(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    _change_command_status_command(ctx, msg, args, false).await
}

/// Enables a disabled command on this guild.
///
/// Usage: `config guild enable_command urban`
#[command]
#[min_args(1)]
async fn enable_command(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    _change_command_status_command(ctx, msg, args, true).await
}
