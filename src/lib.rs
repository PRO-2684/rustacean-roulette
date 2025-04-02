mod commands;
mod constants;

pub use commands::Commands;
use frankenstein::{
    client_reqwest::Bot, methods::{DeleteMyCommandsParams, SetMyCommandsParams, SetMyDefaultAdministratorRightsParams}, types::BotCommandScope, AsyncTelegramApi, Error
};
use rand::{Rng, seq::index::sample};
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for the bot.
#[derive(Deserialize)]
pub struct Config {
    /// The token for the bot.
    pub token: String,
    /// The configuration for the Russian Roulette game.
    #[serde(default)]
    pub game: RouletteConfig,
    /// The override configuration for groups.
    #[serde(default)]
    pub groups: Vec<GroupConfig>,
}

/// Configuration for the Russian Roulette game.
#[derive(Clone, Debug, Deserialize)]
pub struct RouletteConfig {
    /// Number of chambers in the revolver.
    #[serde(default = "constants::chambers")]
    chambers: usize,
    /// Number of bullets in the revolver.
    #[serde(default = "constants::bullets")]
    bullets: usize,
    /// Minimum time to mute in seconds.
    #[serde(default = "constants::min_mute_time")]
    min_mute_time: u32,
    /// Maximum time to mute in seconds.
    #[serde(default = "constants::max_mute_time")]
    max_mute_time: u32,
}

impl RouletteConfig {
    /// Starts a new game of Russian Roulette.
    pub fn start(self) -> Result<Roulette, &'static str> {
        // Sanity check
        if self.chambers <= 0 {
            return Err("Number of chambers must be greater than 0");
        }
        if self.bullets <= 0 {
            return Err("Number of bullets must be greater than 0");
        }
        if self.bullets > self.chambers {
            return Err("Number of bullets must be less than or equal to number of chambers");
        }
        if self.min_mute_time < 30 {
            return Err("Minimum mute time must be greater than or equal to 30 seconds");
        }
        if self.max_mute_time > 3600 {
            // FIXME: 365 days
            return Err("Maximum mute time must be less than or equal to 3600 seconds");
        }
        if self.min_mute_time > self.max_mute_time {
            return Err("Minimum mute time must be less than or equal to maximum mute time");
        }

        // Initialize the contents of the chambers
        let contents = vec![false; self.chambers];
        let mut roulette = Roulette {
            config: self,
            contents,
            position: 0,
        };
        roulette.restart();

        Ok(roulette)
    }

    /// Get the number of bullets and chambers.
    pub fn info(&self) -> (usize, usize) {
        (self.bullets, self.chambers)
    }

    /// Generate a random mute time and the time until which the user will be muted.
    pub fn random_mute_until(&self) -> (u64, u64) {
        // Generate a random mute time between min and max
        let mut rng = rand::rng();
        let duration: u64 = rng
            .random_range(self.min_mute_time..=self.max_mute_time)
            .into();
        // Convert to seconds and add to current time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        (duration, now + duration)
    }
}

impl Default for RouletteConfig {
    fn default() -> Self {
        Self {
            chambers: constants::chambers(),
            bullets: constants::bullets(),
            min_mute_time: constants::min_mute_time(),
            max_mute_time: constants::max_mute_time(),
        }
    }
}

/// A Russian Roulette game.
#[derive(Clone, Debug)]
pub struct Roulette {
    /// Configuration for the game.
    pub config: RouletteConfig,
    /// An array of boolean values representing the contents of the chambers. `true` means the chamber is loaded with a bullet, `false` means it is empty.
    contents: Vec<bool>,
    /// The current chamber index.
    position: usize,
}

impl Roulette {
    /// Restart the Russian Roulette game.
    pub fn restart(&mut self) {
        self.position = 0;
        self.contents.fill(false);

        // Randomly choose `bullets` chambers to be loaded with bullets.
        let mut rng = rand::rng();
        let selected = sample(&mut rng, self.contents.len(), self.config.bullets);
        for i in selected {
            self.contents[i] = true;
        }
    }

    /// Get the number of bullets and chambers.
    pub fn info(&self) -> (usize, usize) {
        self.config.info()
    }

    /// Generate a random mute time and the time until which the user will be muted.
    pub fn random_mute_until(&self) -> (u64, u64) {
        self.config.random_mute_until()
    }

    /// Try to fire the current chamber.
    ///
    /// - If the chamber is loaded with a bullet, return `Some(true)`
    /// - If the chamber is empty, return `Some(false)`
    /// - If we have fired all filled chambers, return `None`
    pub fn fire(&mut self) -> Option<bool> {
        if self.peek().0 == 0 {
            // No filled chambers left
            return None;
        }

        let result = self.contents[self.position];
        self.position += 1;

        Some(result)
    }

    /// Peek the left-over chambers, returning count of filled and left chambers.
    pub fn peek(&self) -> (usize, usize) {
        let filled = self
            .contents
            .iter()
            .skip(self.position)
            .filter(|&&x| x)
            .count();
        let left = self.contents.len() - self.position;
        (filled, left)
    }
}

/// Configuration for a group.
#[derive(Debug, Deserialize)]
pub struct GroupConfig {
    /// The ID of the group.
    pub id: i64,
    /// Override number of chambers in the revolver.
    chambers: Option<usize>,
    /// Override number of bullets in the revolver.
    bullets: Option<usize>,
    /// Override minimum time to mute in seconds.
    min_mute_time: Option<u32>,
    /// Override maximum time to mute in seconds.
    max_mute_time: Option<u32>,
}

impl GroupConfig {
    /// Resolves to a [`RouletteConfig`].
    pub fn resolve(&self, default: &RouletteConfig) -> RouletteConfig {
        let Self {
            chambers,
            bullets,
            min_mute_time,
            max_mute_time,
            ..
        } = self;
        let (chambers, bullets, min_mute_time, max_mute_time) = (
            chambers.unwrap_or(default.chambers),
            bullets.unwrap_or(default.bullets),
            min_mute_time.unwrap_or(default.min_mute_time),
            max_mute_time.unwrap_or(default.max_mute_time),
        );
        RouletteConfig {
            chambers,
            bullets,
            min_mute_time,
            max_mute_time,
        }
    }
}

/// Set commands and default admin rights for the bot.
pub async fn init_commands_and_rights(bot: &Bot) -> Result<(), Error> {
    let delete_param = DeleteMyCommandsParams::builder().build();
    bot.delete_my_commands(&delete_param).await?;

    let commands_param = SetMyCommandsParams::builder()
        .commands(Commands::list())
        .scope(BotCommandScope::AllGroupChats)
        .build();
    bot.set_my_commands(&commands_param).await?;

    let rights_param = SetMyDefaultAdministratorRightsParams::builder()
        .rights(constants::RECOMMENDED_ADMIN_RIGHTS)
        .build();
    bot.set_my_default_administrator_rights(&rights_param)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fire() {
        let config = RouletteConfig {
            chambers: 3,
            bullets: 1,
            min_mute_time: 60,
            max_mute_time: 600,
        };
        // let mut roulette = config.start().unwrap();
        let mut roulette = Roulette {
            config,
            contents: vec![false, true, false],
            position: 0,
        };

        assert_eq!(roulette.fire(), Some(false));
        assert_eq!(roulette.fire(), Some(true));
        assert_eq!(roulette.fire(), None);
        assert_eq!(roulette.fire(), None);
    }

    #[test]
    fn test_restart() {
        let mut roulette = RouletteConfig::default().start().unwrap();

        for _ in 0..10 {
            roulette.restart();
        }

        assert_eq!(roulette.contents.len(), 6);
        assert_eq!(roulette.peek().0, 2);
        assert_eq!(roulette.position, 0);
    }
}
