mod peek;
mod roulette;

use super::{Roulette, RouletteConfig};
use frankenstein::{client_reqwest::Bot, types::Message};
use peek::PeekCommand;
use roulette::RouletteCommand;
use tokio::sync::Mutex;

/// A command.
pub trait Command {
    /// Trigger word.
    const TRIGGER: &'static str;
    /// Help message.
    const HELP: &'static str;
    /// Execute the command.
    async fn execute(
        bot: &Bot,
        msg: Message,
        roulette: &Mutex<Roulette>,
        roulette_config: &RouletteConfig,
    ) -> Option<String>;
}

/// List of commands. Cheap to clone.
#[non_exhaustive]
pub enum Commands {
    Peek,
    Roulette,
}

impl Commands {
    /// Try to parse the given text to a command.
    ///
    /// # Arguments
    ///
    /// - `text` - The text to check.
    /// - `username` - The username of the bot.
    pub fn parse_command(text: Option<&String>, username: &str) -> Option<Commands> {
        let Some(text) = text else {
            return None;
        };
        let text = text.trim();
        let (command, _arg) = text.split_once(' ').unwrap_or((text, ""));

        // Two possible command formats:
        // 1. /command <arg>
        // 2. /command@bot_username <arg>

        // Trim the leading slash
        let slash = command.starts_with('/');
        if !slash {
            return None;
        }
        let command = &command[1..];

        // Split out the mention and check if it's the bot
        let (command, mention) = command.split_once('@').unwrap_or((command, ""));
        if !mention.is_empty() && mention != username {
            return None;
        }

        // Match the command
        match command {
            PeekCommand::TRIGGER => Some(Commands::Peek),
            RouletteCommand::TRIGGER => Some(Commands::Roulette),
            _ => None,
        }
    }

    /// Execute the command.
    pub async fn execute(
        &self,
        bot: &Bot,
        msg: Message,
        roulette: &Mutex<Roulette>,
        roulette_config: &RouletteConfig,
    ) -> Option<String> {
        match self {
            Self::Peek => PeekCommand::execute(bot, msg, roulette, roulette_config).await,
            Self::Roulette => RouletteCommand::execute(bot, msg, roulette, roulette_config).await,
        }
    }
}
