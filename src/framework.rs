use crate::{
    commands::{
        configuration::*, economy::*, fun::*, hydrate::*, meta::*, moderation::*, music::*,
        reddit::*, roleplay::*,
    },
    database::*,
    MongoClient,
};

use itconfig::*;
use serenity::{
    framework::standard::{
        help_commands,
        macros::{group, help, hook},
        Args, CommandGroup, CommandResult, DispatchError, HelpOptions, Reason, StandardFramework,
    },
    model::{channel::Message, id::UserId},
    prelude::Context,
    utils::Colour,
};
use std::{clone::Clone, collections::HashSet};
use tracing::{debug, error, info};

#[group("Master")]
#[sub_groups(Meta, Fun, Music, Mod)]
struct Master;

// The basic commands group is being defined here.
// this group includes the commands that basically every bot has, nothing really special.
#[group("Meta")]
#[description = "All the basic commands that basically every bot has."]
#[commands(ping, invite, about)]
struct Meta;

// The FUN command group.
// Where all the random commands goes into.
#[group("Fun")]
#[description = "All the random and fun commands."]
#[commands(
    profile,
    qr,
    urban,
    dictionary,
    translate,
    duck_duck_go,
    calculator,
    poll,
    chuck,
    dice,
    uwufy,
    fact,
    why,
    eightball,
    custom_commands,
    self_roles,
    valorant
)]
struct Fun;

// The Roleplay command group.
// Where all the random roleplay goes into.
#[group("Roleplay")]
#[description = "All the fun roleplay commands."]
#[commands(baka, cuddle, hug, kiss, pat, poke, slap, smug, tickle)]
struct Roleplay;

// The Economy command group.
// Where all the economy commands goes into.
#[group("Economy")]
#[description = "All the fun economy related commands."]
#[commands(balance, daily, gamble, leaderboard)]
struct Economy;

// The moderation command group.
#[group("Moderation")]
#[description = "All the moderation related commands."]
#[commands(kick, ban, clear)]
struct Mod;

// The reddit command group.
#[group("Reddit")]
#[description = "All the reddit related commands."]
#[commands(meme, reddit_image, reddit_text)]
struct Reddit;

// The Hydrate command group.
#[group("Hydrate")]
#[description = "All the hydrate reminder related commands."]
#[commands(hydrate)]
struct Hydrate;

// The music command group.
#[group("Music")]
#[description = "All the voice and music related commands."]
#[only_in("guilds")]
#[commands(
    join,
    leave,
    play,
    pause,
    resume,
    stop,
    skip,
    shuffle,
    queue,
    clear_queue,
    repeat,
    remove,
    now_playing
)]
struct Music;

// The configuration command.
// Technically a group, but it only has a single command.
#[group("Configuration")]
#[description = "All the configuration related commands.
Basic usage:
`config user VALUE DATA`
`config guild VALUE DATA`"]
#[prefixes("config", "configure")]
#[commands(guild, user)]
struct Configuration;

// This is a custom help command.
#[help]
#[individual_command_tip = "Hello!
If youd like to get more information about a specific command or group, you can just pass it as a command argument.
All the command examples through out the help will be shown without prefix, use whatever prefix is configured on the server instead.
You can react with 🚫 on *any* message sent by the bot to delete it.\n"]
#[command_not_found_text = "Could not find: `{}`."]
#[strikethrough_commands_tip_in_dm = "~~`Strikethrough commands`~~ are unavailabe because the bot is unable to run them."]
#[strikethrough_commands_tip_in_guild = "~~`Strikethrough commands`~~ are unavailabe because the bot is unable to run them."]
#[max_levenshtein_distance(3)]
#[lacking_permissions = "Hide"]
#[lacking_role = "Hide"]
#[wrong_channel = "Strike"]
#[group_prefix = "Prefix commands"]
async fn my_help(
    ctx: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let mut ho = help_options.clone();
    ho.embed_error_colour = Colour::from_rgb(255, 30, 30);
    ho.embed_success_colour = Colour::from_rgb(141, 91, 255);
    help_commands::with_embeds(ctx, msg, args, &ho, groups, owners).await;
    Ok(())
}

// This is for errors that happen before command execution.
#[hook]
async fn on_dispatch_error(ctx: &Context, msg: &Message, error: DispatchError) {
    match error {
        // Notify the user if the reason of the command failing to execute was because of insufficient arguments.
        DispatchError::NotEnoughArguments { min, given } => {
            let s = {
                if given == 0 && min == 1 {
                    format!("I need an argument to run this command")
                } else if given == 0 {
                    format!("I need atleast {} arguments to run this command", min)
                } else {
                    format!(
                        "I need {} arguments to run this command, but i was only given {}.",
                        min, given
                    )
                }
            };
            let _ = msg.channel_id.say(ctx, s).await;
        }
        DispatchError::IgnoredBot {} => {
            return;
        }
        DispatchError::CheckFailed(_, reason) => {
            if let Reason::User(r) = reason {
                let _ = msg.channel_id.say(ctx, r).await;
            }
        }
        DispatchError::Ratelimited(x) => {
            let _ = msg
                .reply(
                    ctx,
                    format!(
                        "You can't run this command for {} more seconds.",
                        x.as_secs()
                    ),
                )
                .await;
        }
        _ => {
            error!("Unhandled dispatch error: {:?}", error);
        }
    }
}

// This function executes before a command is called.
#[hook]
async fn before(ctx: &Context, msg: &Message, cmd_name: &str) -> bool {
    if let Some(guild_id) = msg.guild_id {
        let data_read = ctx.data.read().await;
        let client = data_read.get::<MongoClient>().unwrap();
        if let Ok(guild) = Guild::from_db(client, guild_id.0 as i64).await {
            if guild.disabled_commands.contains(&cmd_name.to_string()) {
                let _ = msg
                    .reply(
                        ctx,
                        "This command has been disabled by an administrtor of this guild.",
                    )
                    .await;
                return false;
            }
        }
    }
    info!("Running command: {}", &cmd_name);
    debug!("Message: {}", &msg.content);
    true
}

// This function executes every time a command finishes executing.
// It's used here to handle errors that happen in the middle of the command.
#[hook]
async fn after(ctx: &Context, msg: &Message, cmd_name: &str, error: CommandResult) {
    if let Err(why) = &error {
        error!("Error while running command {}", &cmd_name);
        error!("{:?}", &error);
        if let Err(_) = msg.channel_id.say(ctx, why).await {
            error!(
                "Unable to send messages on channel id {}",
                &msg.channel_id.0
            );
        };
    }
}

// Small error event that triggers when a command doesn't exist.
// Incase its a guild specific custom command, it is triggered.
#[hook]
async fn unrecognised_command(ctx: &Context, msg: &Message, unrecognised_command_name: &str) {
    if let Some(guild_id) = msg.guild_id {
        let data_read = ctx.data.read().await;
        let client = data_read.get::<MongoClient>().unwrap();
        if let Ok(guild) = Guild::from_db(client, guild_id.0 as i64).await {
            for c in guild.custom_commands.iter() {
                if c.name == unrecognised_command_name {
                    let _ = msg.reply(ctx, &c.reply).await;
                    break;
                }
            }
        }
    }
}

// Dynamic guild specic prefix
#[hook]
async fn dynamic_prefix(ctx: &Context, msg: &Message) -> Option<String> {
    let data = ctx.data.read().await;
    let mut p = get_env_or_default("PREFIX", "!");
    if let Some(id) = &msg.guild_id {
        let client = data.get::<MongoClient>().unwrap();
        if let Ok(guild) = Guild::from_db(client, id.0 as i64).await {
            p = guild.prefix;
        }
    }
    Some(p)
}

// Helper function to build serenity command framework
pub async fn get_framework(owners: HashSet<UserId>, bot_id: UserId) -> StandardFramework {
    StandardFramework::new()
        .configure(|c| {
            c.allow_dm(false)
                .on_mention(Some(bot_id))
                .dynamic_prefix(dynamic_prefix)
                .owners(owners)
                .case_insensitivity(true)
        })
        .before(before)
        .after(after)
        .bucket("reddit", |b| b.delay(5).time_span(5).limit(1))
        .await
        .on_dispatch_error(on_dispatch_error)
        .unrecognised_command(unrecognised_command)
        .group(&META_GROUP)
        .group(&FUN_GROUP)
        .group(&ROLEPLAY_GROUP)
        .group(&ECONOMY_GROUP)
        .group(&REDDIT_GROUP)
        .group(&MUSIC_GROUP)
        .group(&MOD_GROUP)
        .group(&HYDRATE_GROUP)
        .group(&CONFIGURATION_GROUP)
        .help(&MY_HELP)
}
