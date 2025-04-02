mod commands;
mod defaults;

pub use commands::Commands;
use frankenstein::{
    AsyncTelegramApi, Error,
    client_reqwest::Bot,
    methods::{SetMyCommandsParams, SetMyDefaultAdministratorRightsParams},
    types::ChatAdministratorRights,
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
    pub game: Roulette,
    /// The override configuration for groups.
    #[serde(default)]
    pub groups: Vec<GroupConfig>,
}

/// A Russian Roulette game.
#[derive(Clone, Debug, Deserialize)]
pub struct Roulette {
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
    /// An array of boolean values representing the contents of the chambers. `true` means the chamber is loaded with a bullet, `false` means it is empty.
    #[serde(skip)]
    contents: Vec<bool>,
    /// The current chamber index.
    #[serde(skip)]
    position: usize,
}

impl Roulette {
    /// (Re-)Start a new game of Russian Roulette.
    pub fn restart(&mut self) {
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

        self.position = 0;
        self.contents.fill(false);
        self.contents.resize(self.chambers, false);

        // Randomly choose `bullets` chambers to be loaded with bullets.
        let mut rng = rand::rng();
        let selected = sample(&mut rng, self.chambers, self.bullets);
        for i in selected {
            self.contents[i] = true;
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
            .skip(self.position) // FIXME: ?
            .filter(|&&x| x)
            .count();
        let left = self.contents.len() - self.position;
        (filled, left)
    }
}

impl Default for Roulette {
    fn default() -> Self {
        Roulette {
            chambers: defaults::chambers(),
            bullets: defaults::bullets(),
            min_mute_time: defaults::min_mute_time(),
            max_mute_time: defaults::max_mute_time(),
            contents: vec![],
            position: 0,
        }
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
    pub fn resolve(&self, default: &Roulette) -> Roulette {
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
        Roulette {
            chambers,
            bullets,
            min_mute_time,
            max_mute_time,
            ..Default::default()
        }
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
    bot.set_my_default_administrator_rights(&rights_param)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fire() {
        let mut roulette = Roulette {
            contents: vec![false, true, false],
            position: 0,
            chambers: 3,
            bullets: 1,
            ..Default::default()
        };

        assert_eq!(roulette.fire(), Some(false));
        assert_eq!(roulette.fire(), Some(true));
        assert_eq!(roulette.fire(), None);
        assert_eq!(roulette.fire(), None);
    }

    #[test]
    fn test_restart() {
        let mut roulette = Roulette::default();

        roulette.restart();
        assert_eq!(roulette.contents.len(), 6);
        assert_eq!(roulette.peek().0, 2);
        assert_eq!(roulette.position, 0);
    }

    #[test]
    fn test_multi_restart() {
        let mut roulette = Roulette::default();

        for _ in 0..10 {
            roulette.restart();
        }

        assert_eq!(roulette.contents.len(), 6);
        assert_eq!(roulette.peek().0, 2);
        assert_eq!(roulette.position, 0);
    }
}
