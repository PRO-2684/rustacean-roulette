use env_logger::Env;
use frankenstein::{client_reqwest::Bot, methods::{GetChatMemberParams, GetChatParams, GetUpdatesParams}, types::{ChatMember, ChatType, Message}, updates::UpdateContent, AsyncTelegramApi, Error};
use log::{debug, error, info};
use russian_roulette::{Config, RouletteConfig, Roulette, is_roulette};
use std::{collections::HashMap, io::Write};
use toml::de;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
    .format(|buf, record| {
        let level = record.level();
        let style = buf.default_level_style(level);
        writeln!(buf, "[{style}{level}{style:#}] {}", record.args())
    })
    .init();

    // Read path to the config file as the first argument, default to "config.toml"
    let config_path = std::env::args().nth(1).unwrap_or_else(|| "config.toml".to_string());
    let config_toml = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("Failed to read config file: {config_path}"));

    // Parse the config file
    let config: Config = match de::from_str(&config_toml) {
        Ok(config) => config,
        Err(e) => panic!("Failed to parse config file ({config_path}): {e}"),
    };

    // Create a new Telegram Bot
    let bot = Bot::new(&config.token);
    let me = bot.get_me().await?;
    let Some(username) = me.result.username else {
        panic!("Failed to get bot username");
    };

    // setMyDefaultAdministratorRights / setMyCommands
    // TBD

    // Group-wise data (mapping group ID to Roulette instance)
    let mut group_data = HashMap::new();
    for group_id in &config.whitelist {
        // Acquire chat info
        let get_chat_param = GetChatParams::builder()
            .chat_id(*group_id)
            .build();
        let group = bot.get_chat(&get_chat_param).await?.result;
        // Check chat type
        if !matches!(group.type_field, ChatType::Group | ChatType::Supergroup) {
            info!("Group ID: {group_id} is not a group or supergroup, ignoring");
            continue;
        }
        debug!("Group ID: {group_id}, Name: {}", group.title.unwrap_or_else(|| "<unknown>".to_string()));
        // Check permissions
        let get_chat_member_param = GetChatMemberParams::builder()
            .chat_id(*group_id)
            .user_id(me.result.id)
            .build();
        let member = bot.get_chat_member(&get_chat_member_param).await?.result;
        let can_restrict = match member {
            ChatMember::Creator(_) => true,
            ChatMember::Administrator(admin) => {
                admin.can_restrict_members
            }
            _ => false,
        };
        if !can_restrict {
            info!("Bot cannot restrict members in group ID: {group_id}, ignoring");
            continue;
        }

        let roulette = config.game.start();
        group_data.insert(group_id, roulette);
    }

    // Handle incoming messages
    let mut update_params = GetUpdatesParams::builder().build();
    loop {
        match bot.get_updates(&update_params).await {
            Ok(updates) => {
                // Update offset
                let Some(last) = updates.result.last() else {
                    continue;
                };
                update_params.offset.replace((last.update_id + 1).into());
                // Process each update
                for update in updates.result {
                    debug!("Received update: {update:?}");
                    let UpdateContent::Message(msg) = update.content else {
                        continue;
                    };
                    // Whitelist check
                    if config.whitelist.contains(&msg.chat.id) {
                        debug!("Received message from whitelisted chat: {msg:?}");
                    } else {
                        debug!("Received message from non-whitelisted chat: {msg:?}");
                        continue;
                    }

                    let text = msg.text.unwrap_or_default();
                    if is_roulette(&text, &username) {

                    }
                }
            }
            Err(err) => {
                error!("Error getting updates: {err}");
            }
        }
    }
}

async fn handle_roulette() {}
