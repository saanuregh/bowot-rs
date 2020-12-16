use crate::{database::Guild, framework::MASTER_GROUP, utils::checks::*, Database};
use comfy_table::{Cell, CellAlignment::Center, ContentArrangement::Dynamic, Table};
use serenity::{
    collector::MessageCollectorBuilder,
    framework::standard::{macros::command, Args, CommandResult},
    futures::stream::StreamExt,
    model::channel::Message,
    model::id::RoleId,
    prelude::Context,
};
use std::time::Duration;

/// Configures the bot for the guild/server it was invoked on.
///
/// Configurable aspects:
/// `prefix`: Changes the bot prefix.
/// `default_role`: Sets the mute role of the server.
/// `add_custom_command`: Add a custom command.
/// `remove_custom_command`: Remove a custom command.
/// `add_trigger_phrase`: Add a trigger phrase.
/// `remove_trigger_phrase`: Remove a trigger phrase.
/// `add_self_role`: Add a self role.
/// `remove_self_role`: Remove a self role.
/// `disable_command`: Disables a command.
/// `enable_command`: Enables a disabled command.
#[command]
#[required_permissions(MANAGE_GUILD)]
#[only_in("guilds")]
#[aliases(server)]
#[sub_commands(
    prefix,
    default_role,
    add_custom_command,
    remove_custom_command,
    add_trigger_phrase,
    remove_trigger_phrase,
    add_self_role,
    remove_self_role,
    disable_command,
    enable_command
)]
async fn guild(_ctx: &Context, _msg: &Message, _args: Args) -> CommandResult {
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
    let db = data.get::<Database>().unwrap();
    Guild::from_db(db, guild_id)
        .await?
        .change_prefix(prefix.to_string())?
        .save_guild(db)
        .await?;
    msg.reply(
        ctx,
        format!("Successfully changed your prefix to `{}`", prefix),
    )
    .await?;
    Ok(())
}

/// Change the default role given to new members on this guild.
///
/// Usage: `config guild default_role`
#[command]
async fn default_role(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    let dguild = msg.guild(ctx).await.unwrap();
    let mut table = Table::new();
    table.force_no_tty().enforce_styling();
    table.set_content_arrangement(Dynamic).set_table_width(100);
    table.set_header(vec![
        Cell::new("Role id").set_alignment(Center),
        Cell::new("Role name").set_alignment(Center),
    ]);
    for (role_id, role) in dguild.roles.clone() {
        if !role.name.starts_with("@") {
            let _role = role_id.0 as i64;
            table.add_row(vec![
                Cell::new(_role).set_alignment(Center),
                Cell::new(role.name).set_alignment(Center),
            ]);
        }
    }
    let role_msg = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title("Select a role");
                e.description(format!("```\n{}\n```", table));
                e.footer(|f| f.text("Reply with any of the following role id in 10 seconds"))
            })
        })
        .await?;
    let collected_msg = MessageCollectorBuilder::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .collect_limit(1u32)
        .timeout(Duration::from_secs(10))
        .await
        .collect::<Vec<_>>()
        .await;
    if collected_msg.len() == 1 {
        let reply_role_id = RoleId(collected_msg[0].content.parse::<u64>()?);
        for (role_id, role) in dguild.roles {
            if reply_role_id == role_id {
                Guild::from_db(db, guild_id)
                    .await?
                    .change_default_role(reply_role_id)?
                    .save_guild(db)
                    .await?;
                msg.reply(
                    ctx,
                    format!("Successfully changed your default_role to `{}`", role.name),
                )
                .await?;
                return Ok(());
            }
        }
        msg.reply(
            ctx,
            format!(
                "Couldn't find the specified role with id `{}`",
                reply_role_id
            ),
        )
        .await?;
        return Ok(());
    }
    role_msg.delete(ctx).await?;
    msg.reply(ctx, "Didn't choose role, try again later!")
        .await?;
    Ok(())
}

fn _command_exist(command_name: String) -> bool {
    for group in MASTER_GROUP.options.sub_groups {
        for command in group.options.commands {
            if command.options.names.contains(&command_name.as_str()) {
                return true;
            }
        }
    }
    false
}

/// Add a custom command on this guild.
///
/// Usage: `config guild add_custom_command <command> <reply>`
/// Usage: `config guild add_custom_command hello world`
#[command]
#[min_args(2)]
async fn add_custom_command(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let cmd = args.single::<String>()?;
    if _command_exist(cmd.clone()) {
        msg.reply(ctx, format!("Command already exist `{}`", cmd.clone()))
            .await?;
        return Ok(());
    }
    let reply = args.rest();
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    Guild::from_db(db, guild_id)
        .await?
        .add_custom_command(cmd.clone(), reply.to_string())?
        .save_guild(db)
        .await?;
    msg.reply(ctx, format!("Successfully added custom command `{}`", cmd))
        .await?;
    Ok(())
}

/// Remove a custom command on this guild.
///
/// Usage: `config guild remove_custom_command <command>`
/// Usage: `config guild remove_custom_command hello`
#[command]
#[min_args(1)]
async fn remove_custom_command(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let cmd = args.single::<String>()?;
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    Guild::from_db(db, guild_id)
        .await?
        .remove_custom_command(cmd.clone())?
        .save_guild(db)
        .await?;
    msg.reply(
        ctx,
        format!("Successfully removed custom command `{}`", cmd),
    )
    .await?;
    Ok(())
}

/// Add a trigger phrase on this guild.
///
/// Usage: `config guild add_trigger_phrase <trigger> <reply>`
/// Usage: `config guild add_trigger_phrase hello world`
#[command]
#[min_args(2)]
async fn add_trigger_phrase(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let phrase = args.single::<String>()?;
    let reply = args.rest();
    let mut emote = ' ';
    let react_msg = msg
        .reply(
            ctx,
            "React with the reaction to add a reaction as well, you got 10 seconds!",
        )
        .await
        .unwrap();
    if let Some(reaction) = &react_msg
        .await_reaction(&ctx)
        .timeout(Duration::from_secs(10))
        .author_id(msg.author.id)
        .await
    {
        if let Some(e) = reaction.as_inner_ref().emoji.as_data().chars().next() {
            emote = e;
        }
    }
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    Guild::from_db(db, guild_id)
        .await?
        .add_trigger_phrase(phrase.clone(), reply.to_string(), emote)?
        .save_guild(db)
        .await?;
    msg.reply(
        ctx,
        format!("Successfully added trigger phrase `{}`", phrase),
    )
    .await?;
    Ok(())
}

/// Add a trigger phrase on this guild.
///
/// Usage: `config guild remove_trigger_phrase <trigger>`
/// Usage: `config guild remove_trigger_phrase hello`
#[command]
#[min_args(1)]
async fn remove_trigger_phrase(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let phrase = args.single::<String>()?;
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    Guild::from_db(db, guild_id)
        .await?
        .remove_trigger_phrase(phrase.clone())?
        .save_guild(db)
        .await?;
    msg.reply(
        ctx,
        format!("Successfully removed trigger phrase `{}`", phrase),
    )
    .await?;
    Ok(())
}

/// Disables a command on this guild.
///
/// Usage: `config guild disable_command urban`
#[command]
#[min_args(1)]
async fn disable_command(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let command_name = args.single_quoted::<String>()?;
    if _command_exist(command_name.clone()) {
        let data = ctx.data.read().await;
        let db = data.get::<Database>().unwrap();
        Guild::from_db(db, msg.guild_id.unwrap())
            .await?
            .add_disabled_command(command_name.clone())?
            .save_guild(db)
            .await?;
        msg.reply(
            ctx,
            format!("Command `{}` successfully disabled", command_name),
        )
        .await?;

        return Ok(());
    }
    msg.reply(ctx, "Command not found").await?;
    Ok(())
}

/// Enables a disabled command on this guild.
///
/// Usage: `config guild enable_command urban`
#[command]
#[min_args(1)]
async fn enable_command(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let command_name = args.single_quoted::<String>()?;
    if _command_exist(command_name.clone()) {
        let data = ctx.data.read().await;
        let db = data.get::<Database>().unwrap();
        Guild::from_db(db, msg.guild_id.unwrap())
            .await?
            .remove_disabled_command(command_name.clone())?
            .save_guild(db)
            .await?;
        msg.reply(
            ctx,
            format!("Command `{}` successfully re-enabled", command_name),
        )
        .await?;
        return Ok(());
    }
    msg.reply(ctx, "Command not found").await?;
    Ok(())
}

/// Add a self role on this guild.
///
/// Usage: `config guild add_self_role <role_name>`
/// Usage: `config guild add_self_role uwu`
#[command]
#[checks(bot_has_manage_roles)]
#[min_args(1)]
async fn add_self_role(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let role_name = args.single::<String>()?;
    let role = msg
        .guild_id
        .unwrap()
        .create_role(ctx, |r| {
            r.hoist(true).mentionable(true).name(role_name.clone())
        })
        .await?;
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    Guild::from_db(db, guild_id)
        .await?
        .add_self_role(role.id)?
        .save_guild(db)
        .await?;
    msg.reply(ctx, format!("Successfully added self role `{}`", role_name))
        .await?;
    Ok(())
}

/// Remove a self role on this guild.
///
/// Usage: `config guild remove_self_role <role_name>`
/// Usage: `config guild remove_self_role uwu`
#[command]
#[checks(bot_has_manage_roles)]
async fn remove_self_role(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    let mut guild = Guild::from_db(db, guild_id).await?;
    if guild.self_roles.is_empty() {
        msg.reply(ctx, "There are no self roles").await?;
        return Ok(());
    }
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
        if guild.self_roles.contains(&_role) {
            table.add_row(vec![
                Cell::new(_role).set_alignment(Center),
                Cell::new(role.name).set_alignment(Center),
            ]);
        }
    }
    let role_msg = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title("Select a role");
                e.description(format!("```\n{}\n```", table));
                e.footer(|f| f.text("Reply with any of the following role id in 10 seconds"))
            })
        })
        .await?;
    let collected_msg = MessageCollectorBuilder::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .collect_limit(1u32)
        .timeout(Duration::from_secs(10))
        .await
        .collect::<Vec<_>>()
        .await;
    if collected_msg.len() == 1 {
        let reply_role_id = RoleId(collected_msg[0].content.parse::<u64>()?);
        for (role_id, role) in dguild.roles.clone() {
            if reply_role_id == role_id {
                dguild.delete_role(ctx, role_id).await?;
                guild
                    .remove_self_role(reply_role_id)?
                    .save_guild(db)
                    .await?;
                msg.reply(
                    ctx,
                    format!("Successfully removed self role `{}`", role.name),
                )
                .await?;
                return Ok(());
            }
        }
        msg.reply(
            ctx,
            format!(
                "Couldn't find the specified role with id `{}`",
                reply_role_id
            ),
        )
        .await?;
        return Ok(());
    }
    role_msg.delete(ctx).await?;
    msg.reply(ctx, "Didn't choose role, try again later!")
        .await?;
    Ok(())
}

/// Configures the user specific settings on this guild.
///
/// Configurable aspects:
/// `add_role`: Add a self role to yourself available on this guild.
/// `remove_role`: Remove a self role assigned to yourself.
#[command]
#[aliases("self", "me")]
#[sub_commands(add_role, remove_role)]
async fn user(_ctx: &Context, _msg: &Message, _args: Args) -> CommandResult {
    Ok(())
}

/// Add a self role to yourself available on this guild.
///
/// Usage: `config user add_role`
#[command]
#[checks(bot_has_manage_roles)]
async fn add_role(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    let guild = Guild::from_db(db, guild_id).await?;
    if guild.self_roles.is_empty() {
        msg.reply(ctx, "There are no self roles").await?;
        return Ok(());
    }
    let dguild = msg.guild(ctx).await.unwrap();
    let mut table = Table::new();
    table.force_no_tty().enforce_styling();
    table.set_content_arrangement(Dynamic).set_table_width(100);
    table.set_header(vec![
        Cell::new("Role id").set_alignment(Center),
        Cell::new("Role name").set_alignment(Center),
    ]);
    for (role_id, role) in dguild.roles.clone() {
        let _role = role_id.0 as i64;
        if guild.self_roles.contains(&_role) {
            table.add_row(vec![
                Cell::new(_role).set_alignment(Center),
                Cell::new(role.name).set_alignment(Center),
            ]);
        }
    }
    let role_msg = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title("Select a role");
                e.description(format!("```\n{}\n```", table));
                e.footer(|f| f.text("Reply with any of the following role id in 10 seconds"))
            })
        })
        .await?;
    let collected_msg = MessageCollectorBuilder::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .collect_limit(1u32)
        .timeout(Duration::from_secs(10))
        .await
        .collect::<Vec<_>>()
        .await;
    if collected_msg.len() == 1 {
        let reply_role_id = RoleId(collected_msg[0].content.parse::<u64>()?);
        for (role_id, role) in dguild.roles.clone() {
            if reply_role_id == role_id {
                let mut member = msg.member(ctx).await.unwrap();
                member.add_role(ctx, role_id).await?;
                msg.reply(ctx, format!("Successfully added self role `{}`", role.name))
                    .await?;
                return Ok(());
            }
        }
        msg.reply(
            ctx,
            format!(
                "Couldn't find the specified role with id `{}`",
                reply_role_id
            ),
        )
        .await?;
        return Ok(());
    }
    role_msg.delete(ctx).await?;
    msg.reply(ctx, "Didn't choose role, try again later!")
        .await?;
    Ok(())
}

/// Remove a self role assigned to yourself.
///
/// Usage: `config user remove_role`
#[command]
#[checks(bot_has_manage_roles)]
async fn remove_role(ctx: &Context, msg: &Message) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let data = ctx.data.read().await;
    let db = data.get::<Database>().unwrap();
    let guild = Guild::from_db(db, guild_id).await?;
    let mut table = Table::new();
    table.force_no_tty().enforce_styling();
    table.set_content_arrangement(Dynamic).set_table_width(100);
    table.set_header(vec![
        Cell::new("Role id").set_alignment(Center),
        Cell::new("Role name").set_alignment(Center),
    ]);
    let mut member = msg.member(ctx).await.unwrap();
    for role_id in member.roles.clone() {
        let _role = role_id.0 as i64;
        if guild.self_roles.contains(&_role) {
            let role = role_id.to_role_cached(ctx).await.unwrap();
            table.add_row(vec![
                Cell::new(_role).set_alignment(Center),
                Cell::new(role.name).set_alignment(Center),
            ]);
        }
    }
    let role_msg = msg
        .channel_id
        .send_message(ctx, |m| {
            m.embed(|e| {
                e.title("Select a role");
                e.description(format!("```\n{}\n```", table));
                e.footer(|f| f.text("Reply with any of the following role id in 10 seconds"))
            })
        })
        .await?;
    let collected_msg = MessageCollectorBuilder::new(&ctx)
        .author_id(msg.author.id)
        .channel_id(msg.channel_id)
        .collect_limit(1u32)
        .timeout(Duration::from_secs(10))
        .await
        .collect::<Vec<_>>()
        .await;
    if collected_msg.len() == 1 {
        let reply_role_id = RoleId(collected_msg[0].content.parse::<u64>()?);
        for role_id in member.roles.clone() {
            if reply_role_id == role_id {
                let role = role_id.to_role_cached(ctx).await.unwrap();
                member.remove_role(ctx, role_id).await?;
                msg.reply(
                    ctx,
                    format!("Successfully removed self role `{}`", role.name),
                )
                .await?;
                return Ok(());
            }
        }
        msg.reply(
            ctx,
            format!(
                "Couldn't find the specified role with id `{}`",
                reply_role_id
            ),
        )
        .await?;
        return Ok(());
    }
    role_msg.delete(ctx).await?;
    msg.reply(ctx, "Didn't choose role, try again later!")
        .await?;
    Ok(())
}
