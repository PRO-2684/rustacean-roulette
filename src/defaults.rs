//! Default values for the game.

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
