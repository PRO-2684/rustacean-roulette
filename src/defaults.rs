//! Default values for the game.

use super::RouletteConfig;

/// Default configuration for the Russian Roulette game.
pub fn default_config() -> RouletteConfig {
    RouletteConfig {
        chambers: chambers(),
        bullets: bullets(),
        min_mute_time: min_mute_time(),
        max_mute_time: max_mute_time(),
    }
}

/// Default number of chambers in the revolver.
pub fn chambers() -> usize {
    6
}

/// Default number of bullets in the revolver.
pub fn bullets() -> usize {
    2
}

/// Default minimum time to mute in seconds.
pub fn min_mute_time() -> u32 {
    60
}

/// Default maximum time to mute in seconds.
pub fn max_mute_time() -> u32 {
    600
}
