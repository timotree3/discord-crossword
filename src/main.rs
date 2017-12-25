extern crate chrono;
extern crate chrono_tz;
extern crate env_logger;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serenity;

mod puzzles;
mod discord;

use puzzles::Puzzle;

use serenity::model::*;
use serenity::prelude::{Client, Context, EventHandler};

use error_chain::ChainedError;

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{}
}

use errors::*;

const CHECKMARK: char = '✅'; // '\u{2705}'

struct Handler;
impl EventHandler for Handler {
    fn on_ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        std::thread::spawn(move || Puzzle::every(|new| announce_in_all(new)));
    }
    fn on_reaction_add(&self, _: Context, reaction: Reaction) {
        debug!("new reaction: {:?}", reaction.emoji);
        process_reaction(&reaction).unwrap_or_else(|e| {
            warn!(
                "failed to process reaction ({:?}): {}",
                reaction,
                e.display_chain()
            )
        });
        fn process_reaction(reaction: &Reaction) -> Result<()> {
            if reaction.emoji != CHECKMARK.into() {
                debug!("skipping reaction because it isn't a checkmark");
                return Ok(());
            }

            let channel_lock = match discord::guild_channel(discord::reaction_channel(&reaction)
                .chain_err(|| "failed to get reaction channel")?)
            {
                Some(channel) => channel,
                None => return Ok(()),
            };

            let channel = channel_lock.read().unwrap();
            if channel.name != "crosswords" {
                debug!("skipping reaction because it isn't in #crosswords");
                return Ok(());
            }

            let guild_id = channel.guild_id;

            let (_puzzle_channel_id, puzzle_channel) =
                discord::find_channel(
                    &Puzzle::from_announcement(discord::reaction_message(&reaction)
                        .chain_err(|| "failed to get reaction message")?)
                        .to_channel_name(),
                    guild_id,
                ).chain_err(|| "failed to find puzzle channel")?
                    .chain_err(|| "failed to find puzzle channel")?;

            discord::unhide_channel(&puzzle_channel, discord::from_user_id(reaction.user_id))
                .chain_err(|| "failed to hide channel")?;
            Ok(())
        }
    }

    fn on_reaction_remove(&self, _: Context, reaction: Reaction) {
        debug!("reaction removed: {:?}", reaction.emoji);
    }
}

command!(announce_fake(_context, message) {
    process_message(&message).unwrap_or_else(|e| {
        warn!(
            "failed to process message ({:?}): {}",
            message,
            e.display_chain()
        )
    });

    fn process_message(message: &Message) -> Result<()> {
        if message.is_own() {
            return Ok(());
        }
        let guild_channel_lock = match discord::guild_channel(message.channel_id.get().chain_err(|| "failed to get channel")?) {
            Some(c) => c,
            None => return Ok(()),
        };

        let guild_channel = guild_channel_lock.read().unwrap();

        if guild_channel.name != "commands" {
            return Ok(());
        }

        message.reply("Announcing...").chain_err(|| "failed to send reply message")?;

        Ok(())
    }
});

quick_main!(run);

fn run() -> Result<()> {
    env_logger::init().chain_err(|| "failed to init logger")?;
    let mut client = {
        let token = std::env::var("DISCORD_TOKEN")
            .chain_err(|| "failed to retrieve token from environment")?;
        Client::new(&token, Handler)
    };

    client.with_framework(
        serenity::framework::standard::StandardFramework::new()
            .configure(|c| c.on_mention(true))
            .on("announce", announce_fake),
    );

    info!("starting!");

    client.start().chain_err(|| "failed to start client")?;

    Ok(())
}

fn announce_in_all(new: Puzzle) {
    info!("broadcasting for puzzle: {}", new);
    let guilds = &serenity::CACHE.read().unwrap().guilds;
    guilds.iter().for_each(|(guild_id, _guild)| {
        announce_in(new, *guild_id).unwrap_or_else(|e| {
            warn!(
                "failed to broadcast for guild (guild_id={}): {}",
                guild_id,
                e.display_chain()
            )
        })
    });
}

fn announce_in(puzzle: Puzzle, guild_id: GuildId) -> Result<()> {
    // get crosswords channel first both to avoid iterating over the new channel and to fail faster.
    let (crosswords_id, _crosswords_lock) = discord::find_channel("crosswords", guild_id)
        .chain_err(|| "failed to find #crosswords")?
        .chain_err(|| "failed to find #crosswords")?;

    let _todays_channel =
        discord::create_unique_hidden_channel(&puzzle.to_channel_name(), guild_id)
            .chain_err(|| "failed to create todays hidden channel")?;

    crosswords_id
        .send_message(|m| {
            m.content(&puzzle.to_announcement())
                .reactions(Some(CHECKMARK))
        })
        .chain_err(|| "failed to send announcement message")?;

    Ok(())
}
