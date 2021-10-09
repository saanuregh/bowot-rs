use std::{
    collections::HashSet,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

use poise::{EditTracker, Event, Framework, FrameworkBuilder, FrameworkOptions, PrefixFrameworkOptions, samples::on_error, serenity_prelude::UserId};
use tracing::{error, info};

use crate::{
    commands::{economy, fun, meta, music, reddit, roleplay},
    constants::PREFIX,
    data::{Data, PgPoolContainer},
    database::Guild,
    services::start_services,
    types::{Error, SerenityContext},
};

#[allow(clippy::single_match)]
pub async fn listener<'a>(
    ctx: &SerenityContext,
    event: &Event<'a>,
    _framework: &Framework<Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        Event::Ready { data_about_bot } => {
            info!("{} is ready!", data_about_bot.user.name);
        }
        Event::CacheReady { guilds: _ } => {
            let ctx = Arc::new(ctx.clone());
            if *(crate::constants::ENABLE_SERVICES) {
                if !data.is_services_running.load(Ordering::Relaxed) {
                    start_services(ctx).await;
                    data.is_services_running.swap(true, Ordering::Relaxed);
                    info!("Services started");
                }
            }
        }
        Event::GuildCreate { guild, is_new: _ } => {
            let guild_id = guild.id;
            let data = ctx.data.read().await;
            let db = data.get::<PgPoolContainer>().unwrap();
            let non_bot_members: Vec<i64> = guild
                .members
                .clone()
                .into_iter()
                .filter(|(_id, m)| !m.user.bot)
                .map(|(id, _m)| id.0 as i64)
                .collect();
            let db_guild = Guild::new(db, guild_id);
            if let Err(why) = db_guild.insert().await {
                error!("error adding guild to db {:?}", why);
                return Ok(());
            };

            if let Ok(db_members) = db_guild.get_members().await {
                let db_member_ids: Vec<i64> = db_members.iter().map(|m| m.id).collect();
                for id in non_bot_members {
                    if !db_member_ids.contains(&id) {
                        if let Err(why) = db_guild.insert_member(id).await {
                            error!("error adding member to db guild {:?}", why);
                            return Ok(());
                        };
                    }
                }
            }
        }
        Event::GuildMemberAddition {
            guild_id,
            new_member,
        } => {
            if !new_member.user.bot {
                let member_id = new_member.user.id;
                let data = ctx.data.read().await;
                let db = data.get::<PgPoolContainer>().unwrap();
                let db_guild = Guild::new(db, guild_id.0 as i64);
                if let Ok(member) = db_guild.get_member(member_id).await {
                    if member.is_none() {
                        if let Err(why) = db_guild.insert_member(member_id).await {
                            error!("error adding member to db guild {:?}", why)
                        }
                    }
                }
            }
        }
        _ => return Ok(()),
    }
    Ok(())
}

pub fn get_framework_builder(
    bot_token: String,
    owners: HashSet<UserId>,
) -> FrameworkBuilder<Data, Error> {
    let options: FrameworkOptions<Data, Error> = FrameworkOptions {
        prefix_options: PrefixFrameworkOptions {
            edit_tracker: Some(EditTracker::for_timespan(Duration::from_secs(3600))),
            ..Default::default()
        },
        on_error: |e, ctx| Box::pin(on_error(e, ctx)),
        owners,
        listener: |ctx, event, framework, data| Box::pin(listener(ctx, event, framework, data)),
        ..Default::default()
    };

    Framework::build()
        .prefix(PREFIX.clone())
        .token(bot_token)
        .options(options)
        // Command Initialization
        // Meta
        .command(meta::register(), |f| f.category("Meta"))
        .command(meta::help(), |f| f.category("Meta"))
        .command(meta::about(), |f| f.category("Meta"))
        .command(meta::invite(), |f| f.category("Meta"))
        .command(meta::ping(), |f| f.category("Meta"))
        // Music
        .command(music::join(), |f| f.category("Music"))
        .command(music::leave(), |f| f.category("Music"))
        .command(music::play(), |f| f.category("Music"))
        .command(music::skip(), |f| f.category("Music"))
        .command(music::pause(), |f| f.category("Music"))
        .command(music::resume(), |f| f.category("Music"))
        .command(music::seek(), |f| f.category("Music"))
        .command(music::clear(), |f| f.category("Music"))
        .command(music::now_playing(), |f| f.category("Music"))
        .command(music::queue(), |f| f.category("Music"))
        // Economy
        .command(economy::balance(), |f| f.category("Economy"))
        .command(economy::daily(), |f| f.category("Economy"))
        .command(economy::gamble(), |f| f.category("Economy"))
        .command(economy::leaderboard(), |f| f.category("Economy"))
        // Fun
        .command(fun::chuck(), |f| f.category("Fun"))
        .command(fun::dice(), |f| f.category("Fun"))
        .command(fun::duck_duck_go(), |f| f.category("Fun"))
        .command(fun::eightball(), |f| f.category("Fun"))
        .command(fun::fact(), |f| f.category("Fun"))
        .command(fun::poll(), |f| f.category("Fun"))
        .command(fun::pp(), |f| f.category("Fun"))
        .command(fun::profile(), |f| f.category("Fun"))
        .command(fun::respect(), |f| f.category("Fun"))
        .command(fun::ship(), |f| f.category("Fun"))
        .command(fun::translate(), |f| f.category("Fun"))
        .command(fun::triggered(), |f| f.category("Fun"))
        .command(fun::urban(), |f| f.category("Fun"))
        .command(fun::uwufy(), |f| f.category("Fun"))
        .command(fun::why(), |f| f.category("Fun"))
        // Reddit
        .command(reddit::meme(), |f| f.category("Reddit"))
        .command(reddit::reddit_image(), |f| f.category("Reddit"))
        .command(reddit::reddit_text(), |f| f.category("Reddit"))
        // Roleplay
        .command(roleplay::baka(), |f| f.category("Roleplay"))
        .command(roleplay::cuddle(), |f| f.category("Roleplay"))
        .command(roleplay::hug(), |f| f.category("Roleplay"))
        .command(roleplay::kiss(), |f| f.category("Roleplay"))
        .command(roleplay::pat(), |f| f.category("Roleplay"))
        .command(roleplay::poke(), |f| f.category("Roleplay"))
        .command(roleplay::slap(), |f| f.category("Roleplay"))
        .command(roleplay::smug(), |f| f.category("Roleplay"))
        .command(roleplay::tickle(), |f| f.category("Roleplay"))
}
