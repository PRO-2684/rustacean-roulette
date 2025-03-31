use env_logger::Env;
use frankenstein::{
    AsyncTelegramApi, Error,
    client_reqwest::Bot,
    methods::{GetChatMemberParams, GetChatParams, GetUpdatesParams, SendMessageParams},
    types::{ChatMember, ChatType, ReplyParameters},
    updates::UpdateContent,
};
use log::{debug, error, info};
use rustacean_roulette::{Commands, Config, Roulette, RouletteConfig};
use std::{collections::HashMap, io::Write};
use tokio::sync::Mutex;
use toml::de;

#[tokio::main]
async fn main() -> Result<(), Error> {
    setup_logger();

    let Config {
        token,
        whitelist,
        game: roulette_config,
    } = read_config();
    let roulette_config = Box::leak(Box::new(roulette_config)).downgrade();

    // Create a new Telegram Bot
    let bot = Box::leak(Box::new(Bot::new(&token))).downgrade();
    let me = bot.get_me().await?.result;
    let Some(username) = me.username else {
        panic!("Failed to get bot username");
    };

    // TODO: setMyDefaultAdministratorRights / setMyCommands

    let group_data = init_group_data(bot, me.id, &whitelist, &roulette_config)
        .await
        .downgrade();
    info!("Bot started, username: {username}");

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
                    if whitelist.contains(&msg.chat.id) {
                        debug!("Received message from whitelisted chat: {msg:?}");
                    } else {
                        debug!("Received message from non-whitelisted chat: {msg:?}");
                        continue;
                    }

                    let text = msg.text.as_ref();
                    let Some(command) = Commands::parse_command(text, &username) else {
                        debug!("Not a command: {text:?}");
                        continue;
                    };
                    tokio::spawn(async move {
                        let chat_id = msg.chat.id;
                        let message_id = msg.message_id;
                        let roulette = group_data.get(&chat_id).unwrap();
                        let reply = command.execute(bot, msg, roulette, roulette_config).await;
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

fn setup_logger() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
        .format(|buf, record| {
            let level = record.level();
            let style = buf.default_level_style(level);
            writeln!(buf, "[{style}{level}{style:#}] {}", record.args())
        })
        .init();
}

/// Downgrade a mutable reference to an immutable one.
trait Downgrade {
    fn downgrade(self: &mut Self) -> &Self;
}

impl<T> Downgrade for T {
    fn downgrade(self: &mut Self) -> &Self {
        self
    }
}

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

async fn init_group_data(
    bot: &Bot,
    user_id: u64,
    whitelist: &[i64],
    game: &RouletteConfig,
) -> &'static mut HashMap<i64, Mutex<Roulette>> {
    // Group-wise data (mapping group ID to Roulette instance)
    let group_data = Box::leak(Box::new(HashMap::new()));
    for group_id in whitelist {
        // Acquire chat info
        let get_chat_param = GetChatParams::builder().chat_id(*group_id).build();
        let group = match bot.get_chat(&get_chat_param).await {
            Ok(res) => res.result,
            Err(err) => {
                error!("Failed to get chat info for group ID {group_id}: {err}");
                continue;
            }
        };
        // Check chat type
        if !matches!(group.type_field, ChatType::Group | ChatType::Supergroup) {
            info!("Group ID: {group_id} is not a group or supergroup, ignoring");
            continue;
        }
        debug!(
            "Group ID: {group_id}, Name: {}",
            group.title.unwrap_or_else(|| "<unknown>".to_string())
        );
        // Check permissions
        let get_chat_member_param = GetChatMemberParams::builder()
            .chat_id(*group_id)
            .user_id(user_id)
            .build();
        let member = match bot.get_chat_member(&get_chat_member_param).await {
            Ok(res) => res.result,
            Err(err) => {
                error!("Failed to get chat member info for group ID {group_id}: {err}");
                continue;
            }
        };
        let can_restrict = match member {
            ChatMember::Creator(_) => true,
            ChatMember::Administrator(admin) => admin.can_restrict_members,
            _ => false,
        };
        if !can_restrict {
            info!("Bot cannot restrict members in group ID: {group_id}, ignoring");
            continue;
        }

        let roulette = game.start();
        group_data.insert(*group_id, Mutex::new(roulette));
    }

    group_data
}
