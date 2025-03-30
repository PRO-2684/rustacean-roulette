use rand::seq::index::sample;
use serde::Deserialize;

mod defaults;

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
#[derive(Deserialize)]
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
}

/// An on-going game of Russian Roulette.
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
    /// - If the chamber is out of bounds, return `None`
    pub fn fire(&mut self) -> Option<bool> {
        if self.current_chamber >= self.chambers.len() {
            return None;
        }

        let result = self.chambers[self.current_chamber];
        self.current_chamber += 1;

        Some(result)
    }
}

/// Check if the given text is a command to participate in Russian Roulette.
///
/// # Arguments
///
/// - `text` - The text to check.
/// - `username` - The username of the bot.
pub fn is_roulette(text: &str, username: &str) -> bool {
    let text = text.trim();
    let (command, _arg) = text.split_once(' ').unwrap_or((text, ""));

    // Two possible command formats:
    // 1. /command <arg>
    // 2. /command@bot_username <arg>

    // Trim the leading slash
    let slash = command.starts_with('/');
    if !slash {
        return false;
    }
    let command = &command[1..];

    // Split out the mention and check if it's the bot
    let (command, mention) = command.split_once('@').unwrap_or((command, ""));
    if !mention.is_empty() && mention != username {
        return false;
    }

    // Check if the command is "roulette", "russian_roulette", or "rr"
    command == "roulette" || command == "russian_roulette" || command == "rr"
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
