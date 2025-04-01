# `rustacean-roulette`

## Configuration

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
