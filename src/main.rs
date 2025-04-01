use env_logger::Env;
use frankenstein::{
    client_reqwest::Bot, methods::{GetChatMemberParams, GetChatParams, GetUpdatesParams, SendMessageParams}, types::{ChatMember, ChatType, ReplyParameters}, updates::UpdateContent, AsyncTelegramApi, Error
};
use log::{debug, error, info};
use rustacean_roulette::{init_commands_and_rights, Commands, Config, GroupConfig, Roulette};
use std::{collections::HashMap, io::Write};
use tokio::sync::Mutex;
use toml::de;

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_logger();

    let Config {
        token,
        game: default_config,
        groups
    } = read_config();

    // Create a new Telegram Bot
    let bot: &_ = Box::leak(Box::new(Bot::new(&token)));
    let me = bot.get_me().await?.result;
    let Some(username) = me.username else {
        panic!("Failed to get bot username");
    };

    // Set bot commands
    init_commands_and_rights(bot).await?;

    let group_data = init_group_data(bot, me.id, default_config, groups).await;
    let group_data: &_ = Box::leak(Box::new(group_data));
    info!("Bot started: @{username}");

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
                    if group_data.get(&msg.chat.id).is_none() {
                        debug!("Received message from non-whitelisted chat: {msg:?}");
                        continue;
                    }

                    let text = msg.text.as_ref();
                    let Some(command) = Commands::parse(text, &username) else {
                        debug!("Not a command: {text:?}");
                        continue;
                    };
                    tokio::spawn(async move {
                        let chat_id = msg.chat.id;
                        let message_id = msg.message_id;
                        let roulette = group_data.get(&chat_id).unwrap();
                        let reply = command.execute(bot, msg, roulette).await;
                        let Some(reply) = reply else {
                            return;
                        };
                        let reply_param = ReplyParameters::builder().message_id(message_id).build();
                        let send_message_param = SendMessageParams::builder()
                            .chat_id(chat_id)
                            .text(reply)
                            .reply_parameters(reply_param)
                            .build();
                        if let Err(err) = bot.send_message(&send_message_param).await {
                            error!("Failed to send message: {err}");
                        }
                    });
                }
            }
            Err(err) => {
                error!("Error getting updates: {err}");
            }
        }
    }
}

/// Setup the logger.
fn setup_logger() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            let level = record.level();
            let style = buf.default_level_style(level);
            writeln!(buf, "[{style}{level}{style:#}] {}", record.args())
        })
        .init();
}

/// Read the config file and parse it into a `Config` struct.
fn read_config() -> Config {
    // Read path to the config file as the first argument, default to "config.toml"
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_string());
    let config_toml = std::fs::read_to_string(&config_path)
        .unwrap_or_else(|_| panic!("Failed to read config file: {config_path}"));

    // Parse the config file
    let config: Config = match de::from_str(&config_toml) {
        Ok(config) => config,
        Err(e) => panic!("Failed to parse config file ({config_path}): {e}"),
    };

    config
}

/// Initialize group data for the bot.
async fn init_group_data(
    bot: &Bot,
    user_id: u64,
    default_config: Roulette,
    groups: Vec<GroupConfig>,
) -> HashMap<i64, Mutex<Roulette>> {
    // Group-wise data (mapping group ID to Roulette instance)
    let mut group_data = HashMap::new();
    for group_config in groups {
        let group_id = group_config.id;
        // Acquire chat info
        let get_chat_param = GetChatParams::builder().chat_id(group_id).build();
        let group = match bot.get_chat(&get_chat_param).await {
            Ok(res) => res.result,
            Err(err) => {
                error!("Failed to get chat info for group <{group_id}>: {err}");
                continue;
            }
        };
        // Check chat type
        if !matches!(group.type_field, ChatType::Supergroup) {
            info!("Group <{group_id}> is not a supergroup, ignoring");
            continue;
        }
        // Check permissions
        let get_chat_member_param = GetChatMemberParams::builder()
            .chat_id(group_id)
            .user_id(user_id)
            .build();
        let member = match bot.get_chat_member(&get_chat_member_param).await {
            Ok(res) => res.result,
            Err(err) => {
                error!("Failed to get chat member info for group <{group_id}>: {err}");
                continue;
            }
        };
        let can_restrict = match member {
            ChatMember::Creator(_) => true,
            ChatMember::Administrator(admin) => admin.can_restrict_members,
            _ => false,
        };
        if !can_restrict {
            info!("Bot cannot restrict members in group <{group_id}>, ignoring");
            continue;
        }

        // Start a new game for each group
        let mut game = group_config.resolve(&default_config);
        game.restart();
        group_data.insert(group_id, Mutex::new(game));
    }

    group_data
}
