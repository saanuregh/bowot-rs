use futures::{stream, StreamExt};
use regex::Regex;
use serenity::{
    framework::standard::{macros::command, Args, CommandResult},
    model::{
        channel::Message,
        guild::Member,
        id::{MessageId, UserId},
    },
    prelude::Context,
};

async fn parse_member(ctx: &Context, msg: &Message, member_name: String) -> Result<Member, String> {
    let mut members = Vec::new();
    if let Ok(id) = member_name.parse::<u64>() {
        let member = &msg.guild_id.unwrap().member(ctx, id).await;
        match member {
            Ok(m) => Ok(m.to_owned()),
            Err(why) => Err(why.to_string()),
        }
    } else if member_name.starts_with("<@") && member_name.ends_with('>') {
        let re = Regex::new("[<@!>]").unwrap();
        let member_id = re.replace_all(&member_name, "").into_owned();
        let member = &msg
            .guild_id
            .unwrap()
            .member(ctx, UserId(member_id.parse::<u64>().unwrap()))
            .await;
        match member {
            Ok(m) => Ok(m.to_owned()),
            Err(why) => Err(why.to_string()),
        }
    } else {
        let guild = &msg.guild(ctx).await.unwrap();
        let member_name = member_name.split('#').next().unwrap();

        for m in guild.members.values() {
            if m.display_name() == std::borrow::Cow::Borrowed(member_name)
                || m.user.name == member_name
            {
                members.push(m);
            }
        }
        if members.is_empty() {
            let similar_members = &guild.members_containing(&member_name, false, false).await;

            let mut members_string = stream::iter(similar_members.iter())
                .map(|m| async move {
                    let member = &m.0.user;
                    format!("`{}`|", member.name)
                })
                .fold(String::new(), |mut acc, c| async move {
                    acc.push_str(&c.await);
                    acc
                })
                .await;

            let message = {
                if members_string == "" {
                    format!("No member named '{}' was found.", member_name)
                } else {
                    members_string.pop();
                    format!(
                        "No member named '{}' was found.\nDid you mean: {}",
                        member_name, members_string
                    )
                }
            };
            Err(message)
        } else if members.len() == 1 {
            Ok(members[0].to_owned())
        } else {
            let mut members_string = stream::iter(members.iter())
                .map(|m| async move {
                    let member = &m.user;
                    format!("`{}#{}`|", member.name, member.discriminator)
                })
                .fold(String::new(), |mut acc, c| async move {
                    acc.push_str(&c.await);
                    acc
                })
                .await;

            members_string.pop();

            let message = format!(
                "Multiple members with the same name where found: '{}'",
                &members_string
            );
            Err(message)
        }
    }
}

/// Kicks the specified member with an optional reason.
///
/// Usage:
/// `kick @user`
/// `kick "user name"`
/// `kick "user name#3124"`
/// `kick 135423120268984330 he is a very bad person.`
#[command]
#[required_permissions(KICK_MEMBERS)]
#[min_args(1)]
#[only_in("guilds")]
async fn kick(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let member_arg = args.single_quoted::<String>()?;
    let member = parse_member(ctx, &msg, member_arg).await;
    let reason = args.remains();
    match member {
        Ok(m) => {
            if let Some(r) = reason {
                m.kick_with_reason(ctx, r).await?;
            } else {
                m.kick(ctx).await?;
            }
            msg.reply(
                ctx,
                format!(
                    "Successfully kicked member `{}#{}` with id `{}`",
                    m.user.name, m.user.discriminator, m.user.id.0
                ),
            )
            .await?;
        }
        Err(why) => {
            msg.reply(ctx, why.to_string()).await?;
        }
    }

    Ok(())
}

/// Bans the specified member with an optional reason.
///
/// Usage:
/// `ban @user`
/// `ban "user name"`
/// `ban "user name#3124"`
/// `ban 135423120268984330 he is a very bad person.`
#[command]
#[required_permissions(BAN_MEMBERS)]
#[min_args(1)]
#[only_in("guilds")]
async fn ban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let member_arg = args.single_quoted::<String>()?;
    let member = parse_member(ctx, &msg, member_arg).await;
    let reason = args.remains();
    match member {
        Ok(m) => {
            if let Some(r) = reason {
                m.ban_with_reason(ctx, 1, &r).await?;
            } else {
                m.ban(ctx, 1).await?;
            }
            msg.reply(
                ctx,
                format!(
                    "Successfully banned member `{}#{}` with id `{}`",
                    m.user.name, m.user.discriminator, m.user.id.0
                ),
            )
            .await?;
        }
        Err(why) => {
            msg.reply(ctx, why.to_string()).await?;
        }
    }

    Ok(())
}

/// Deletes X number of messages from the current channel.
/// If the messages are older than 2 weeks, due to api limitations, they will not get deleted.
///
/// Usage: `clear 20`
#[command]
#[required_permissions(MANAGE_MESSAGES)]
#[num_args(1)]
#[only_in("guilds")]
#[aliases(purge)]
async fn clear(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let num = args.single::<u64>();
    match num {
        Err(_) => {
            msg.channel_id
                .say(ctx, "The value provided was not a valid number")
                .await?;
        }
        Ok(n) => {
            let channel = &msg.channel(ctx).await.unwrap().guild().unwrap();

            let messages = &channel
                .messages(ctx, |r| r.before(&msg.id).limit(n))
                .await?;
            let messages_ids = messages.iter().map(|m| m.id).collect::<Vec<MessageId>>();

            channel.delete_messages(ctx, messages_ids).await?;

            let success_msg = msg
                .reply(ctx, format!("Successfully deleted `{}` message, This message will self-delete in 5 seconds", n))
                .await?;
            tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
            success_msg.delete(ctx).await?;
            msg.delete(ctx).await?;
        }
    }
    Ok(())
}
