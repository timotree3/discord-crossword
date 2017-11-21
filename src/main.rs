extern crate serenity;
extern crate chrono;
extern crate chrono_tz;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate ctrlc;

mod timings;

use timings::Puzzle;

use chrono::offset::Utc;
use serenity::model::*;
use serenity::prelude::*;

use error_chain::ChainedError;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{}
}

use errors::*;

struct Handler;
impl EventHandler for Handler {
    fn on_ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
    fn on_reaction_add(&self, _: Context, react: Reaction) {
        if react.emoji != ReactionType::Unicode(format!("\u{2705}")) {
            return;
        }

        if let Err(e) = process_checkmark(&react) {
            warn!(
                "failed to process reaction ({:?}): {}",
                react,
                e.display_chain()
            );
        }
    }
}

fn process_checkmark(react: &Reaction) -> Result<()> {
    let channel = react.channel_id.get().chain_err(|| "failed to get channel")?;
    if let Channel::Guild(c) = channel {
        // only care if the checkmark was posted on a guild message
        let channel = c.read().unwrap();
        if channel.name == "crosswords" {
            // only care if the checkmark was posted on a crossword announcement
            let member = channel.guild_id.member(react.user_id).chain_err(
                || "failed to get guild member by id",
            )?;
            // FIXME vvvvvvvvvvvvvvvvvvvvvvvvv
            // member.add_role(--"finished"--);
            info!("ADD ROLE FOR MEMBER : {}", member.user.read().unwrap().name);
        }
    }
    // not a guild channel so don't bother marking as finished.
    Ok(())
}

// fn is_checkmark()

quick_main!(run);

fn run() -> Result<()> {
    env_logger::init().chain_err(|| "failed to init logger")?;
    let mut client = {
        let token = std::env::var("DISCORD_TOKEN").chain_err(
            || "failed to retrieve token from environment",
        )?;
        Client::new(&token, Handler)
    };

    info!("starting!");

    // TODO: broken until CloseHandle is fixed.
    // let closer = client.close_handle();
    // ctrlc::set_handler(move || {
    //     closer.close();
    // }).chain_err(|| "failed to start ctrlc thread")?;

    std::thread::spawn(move || broadcast_loop());

    client.start().chain_err(|| "failed to start client")?;

    Ok(())
}

fn broadcast_loop() {
    loop {
        let current = Puzzle::current_as_of(Utc::now());
        current.wait_until_replaced();
        let new = current.succ();

        if let Err(e) = broadcast(new) {
            warn!(
                "error broadcasting puzzle ({:?}): {}",
                new,
                e.display_chain()
            );
        }
    }
}

fn broadcast(new: Puzzle) -> Result<()> {
    info!("broadcasting for puzzle: {}", new);
    let guilds = serenity::CACHE.read().unwrap().user.guilds().chain_err(
        || "failed to query user guilds",
    )?;
    for g in guilds {
        if let Err(e) = broadcast_guild(new, g.id) {
            warn!(
                "failed to broadcast for guild (id={}): {}",
                g.id,
                e.display_chain()
            )
        }
    }
    Ok(())
}

fn broadcast_guild(puzzle: Puzzle, guild: GuildId) -> Result<()> {
    let channels = guild.channels().chain_err(|| "failed to retrieve channels")?;
    for channel in channels.values() {
        if channel.name == "crosswords" {
            channel
                .say(&format!(
                    "\u{200B}\
                The mini of {} just came out! \
                Play it online at https://nytimes.com/crosswords/game/mini or in the app.\n\
                Once you're done, click the :white_check_mark: below \
                so you can share your thoughts.",
                    puzzle
                ))
                .chain_err(|| "failed to send update message")?;
        }
    }
    Ok(())
}