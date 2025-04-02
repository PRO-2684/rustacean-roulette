# `rustacean-roulette`

🦀 A simple Russian Roulette Telegram bot implemented in Rust.

## Installation

### Using `binstall`

```shell
cargo binstall rustacean-roulette
```

### Downloading from Releases

Navigate to the [Releases page](https://github.com/PRO-2684/rustacean-roulette/releases) and download respective binary for your platform. Make sure to give it execute permissions.

### Compiling from Source

```shell
cargo install rustacean-roulette
```

## Configuration

The configuration file is in [TOML format](https://toml.io/), and it could be placed anywhere you want. An example configuration file is provided below:

```toml
token = "" # Telegram bot token, required

[game] # Game configuration, optional
chambers = 6 # Number of chambers in the revolver
bullets = 2 # Number of bullets in the revolver
min_mute_time = 60 # Minimum mute time in seconds
max_mute_time = 600 # Maximum mute time in seconds

[[groups]] # Whitelisted groups and override configuration
id = 0 # Group ID, required
# Override configuration, identical to game configuration
chambers = 8 # In this group, the revolver has 8 chambers
bullets = 3 # In this group, the revolver has 3 bullets
# ...etc.

[[groups]] # Another group
id = 1 # Group ID, required
# ...Override configuration

# More groups...
```

## Usage

```shell
rustacean-roulette /path/to/config.toml
```

Where `/path/to/config.toml` is the path to your configuration file. Defaults to `./config.toml` if not specified.

## TODO

- Random bullets number
- Probability of gun getting jammed
- `/help`
