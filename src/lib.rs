mod commands;
mod defaults;

pub use commands::Commands;
use frankenstein::{client_reqwest::Bot, methods::{SetMyCommandsParams, SetMyDefaultAdministratorRightsParams}, types::ChatAdministratorRights, AsyncTelegramApi, Error};
use rand::{Rng, seq::index::sample};
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

/// Configuration for the bot.
#[derive(Deserialize)]
pub struct Config {
    /// The token for the bot.
    pub token: String,
    /// List of whitelisted groups.
    #[serde(default)]
    pub whitelist: Vec<i64>,
    /// The configuration for the Russian Roulette game.
    #[serde(default = "defaults::default_config")]
    pub game: RouletteConfig,
}

/// Configuration of the Russian Roulette game.
#[derive(Debug, Deserialize)]
pub struct RouletteConfig {
    /// Number of chambers in the revolver.
    #[serde(default = "defaults::chambers")]
    chambers: usize,
    /// Number of bullets in the revolver.
    #[serde(default = "defaults::bullets")]
    bullets: usize,
    /// Minimum time to mute in seconds.
    #[serde(default = "defaults::min_mute_time")]
    min_mute_time: u32,
    /// Maximum time to mute in seconds.
    #[serde(default = "defaults::max_mute_time")]
    max_mute_time: u32,
}

impl RouletteConfig {
    /// Start a new game of Russian Roulette with the given configuration.
    pub fn start(&self) -> Roulette {
        // Sanity check
        assert!(
            self.chambers > 0,
            "Number of chambers must be greater than 0"
        );
        assert!(self.bullets > 0, "Number of bullets must be greater than 0");
        assert!(
            self.bullets <= self.chambers,
            "Number of bullets must be less than or equal to number of chambers"
        );
        assert!(
            self.min_mute_time >= 30,
            "Minimum mute time must be greater than or equal to 30 seconds"
        );
        assert!(
            self.max_mute_time <= 3600,
            "Maximum mute time must be less than or equal to 3600 seconds"
        ); // FIXME: 365 days
        assert!(
            self.min_mute_time <= self.max_mute_time,
            "Minimum mute time must be less than or equal to maximum mute time"
        );

        // Initialize the chambers with `false` (empty).
        let mut chambers = Vec::with_capacity(self.chambers);
        for _ in 0..self.chambers {
            chambers.push(false);
        }

        // Randomly choose `bullets` chambers to be loaded with bullets.
        let mut rng = rand::rng();
        let selected = sample(&mut rng, self.chambers, self.bullets);
        for i in selected {
            chambers[i] = true;
        }

        Roulette {
            chambers,
            current_chamber: 0,
        }
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

/// An on-going game of Russian Roulette.
#[derive(Debug)]
pub struct Roulette {
    /// An array of boolean values representing the contents of the chambers. `true` means the chamber is loaded with a bullet, `false` means it is empty.
    chambers: Vec<bool>,
    /// The current chamber index.
    current_chamber: usize,
}

impl Roulette {
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

        let result = self.chambers[self.current_chamber];
        self.current_chamber += 1;

        Some(result)
    }

    /// Peek the left-over chambers, returning count of filled and left chambers.
    pub fn peek(&self) -> (usize, usize) {
        let filled = self
            .chambers
            .iter()
            .skip(self.current_chamber)
            .filter(|&&x| x)
            .count();
        let left = self.chambers.len() - self.current_chamber;
        (filled, left)
    }
}

/// Set commands and default admin rights for the bot.
pub async fn init_commands_and_rights(bot: &Bot) -> Result<(), Error> {
    let commands_param = SetMyCommandsParams::builder()
        .commands(Commands::list())
        .build();
    bot.set_my_commands(&commands_param).await?;

    let rights = ChatAdministratorRights::builder()
        .is_anonymous(false)
        .can_manage_chat(false)
        .can_delete_messages(false)
        .can_manage_video_chats(false)
        .can_restrict_members(true) // Required
        .can_promote_members(false)
        .can_change_info(false)
        .can_invite_users(false)
        .build();
    let rights_param = SetMyDefaultAdministratorRightsParams::builder()
        .rights(rights)
        .build();
    bot.set_my_default_administrator_rights(&rights_param).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fire() {
        let mut roulette = Roulette {
            chambers: vec![false, true, false],
            current_chamber: 0,
        };

        assert_eq!(roulette.fire(), Some(false));
        assert_eq!(roulette.fire(), Some(true));
        assert_eq!(roulette.fire(), Some(false));
        assert_eq!(roulette.fire(), None);
    }

    #[test]
    fn test_start() {
        let config = RouletteConfig {
            chambers: 6,
            bullets: 2,
            min_mute_time: 5,
            max_mute_time: 10,
        };

        let roulette = config.start();
        assert_eq!(roulette.chambers.len(), 6);
        assert_eq!(roulette.chambers.iter().filter(|&&x| x).count(), 2);
        assert_eq!(roulette.current_chamber, 0);
    }
}
