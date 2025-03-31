# `russian-roulette`

## Configuration

```toml
token = "" # Telegram bot token, required
whitelist = [] # List of whitelisted group IDs, optional

[game] # Game configuration, optional
chambers = 6 # Number of chambers in the revolver
bullets = 2 # Number of bullets in the revolver
min_mute_time = 60 # Minimum mute time in seconds
max_mute_time = 600 # Maximum mute time in seconds
```
