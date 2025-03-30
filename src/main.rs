use env_logger::Env;
use frankenstein::{
    AsyncTelegramApi, Error,
    client_reqwest::Bot,
    methods::{
        GetChatMemberParams, GetChatParams, GetUpdatesParams, RestrictChatMemberParams,
        SendMessageParams,
    },
    types::{ChatMember, ChatPermissions, ChatType, Message, ReplyParameters},
    updates::UpdateContent,
};
use log::{debug, error, info};
use russian_roulette::{Config, Roulette, RouletteConfig, is_roulette};
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
    let roulette_config = Box::leak(Box::new(roulette_config));

    // Create a new Telegram Bot
    let bot = Box::leak(Box::new(Bot::new(&token)));
    let me = bot.get_me().await?.result;
    let Some(username) = me.username else {
        panic!("Failed to get bot username");
    };

    // TODO: setMyDefaultAdministratorRights / setMyCommands

    let group_data = init_group_data(bot, me.id, &whitelist, &roulette_config).await;

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
                    if is_roulette(text, &username) {
                        tokio::spawn(async {
                            let chat_id = msg.chat.id;
                            let message_id = msg.message_id;
                            let roulette = group_data.get(&chat_id).unwrap();
                            let reply = handle_roulette(bot, msg, roulette, roulette_config).await;
                            let Some(reply) = reply else {
                                return;
                            };
                            let reply_param =
                                ReplyParameters::builder().message_id(message_id).build();
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

async fn handle_roulette(
    bot: &Bot,
    msg: Message,
    roulette: &Mutex<Roulette>,
    roulette_config: &RouletteConfig,
) -> Option<String> {
    const RESTRICTED_PERM: ChatPermissions = ChatPermissions {
        can_send_messages: Some(false),
        can_send_audios: Some(false),
        can_send_documents: Some(false),
        can_send_photos: Some(false),
        can_send_videos: Some(false),
        can_send_video_notes: Some(false),
        can_send_voice_notes: Some(false),
        can_send_polls: Some(false),
        can_send_other_messages: Some(false),
        can_add_web_page_previews: None,
        can_change_info: None,
        can_invite_users: None,
        can_pin_messages: None,
        can_manage_topics: None,
    };
    // Get chat and sender
    let chat = &msg.chat;
    let Some(sender) = &msg.from else {
        error!("Cannot determine sender of message: {msg:?}");
        return None;
    };
    // Determine sender's role
    let get_chat_member_param = GetChatMemberParams::builder()
        .chat_id(chat.id)
        .user_id(sender.id)
        .build();
    let member = match bot.get_chat_member(&get_chat_member_param).await {
        Ok(res) => res.result,
        Err(err) => {
            error!(
                "Failed to get chat member info for user ID {}: {err}",
                sender.id
            );
            return None;
        }
    };
    let is_admin = matches!(
        member,
        ChatMember::Creator(_) | ChatMember::Administrator(_)
    );
    if is_admin {
        return Some("Cannot play roulette as an admin".to_string());
    }
    // Check the roulette status
    let mut roulette = roulette.lock().await;
    let result = match roulette.fire() {
        Some(result) => result,
        None => {
            // Reload the gun
            *roulette = roulette_config.start();
            let (bullets, chambers) = roulette_config.info();
            let send_message_param = SendMessageParams::builder()
                .chat_id(chat.id)
                .text(format!(
                    "The gun has been reloaded, with {bullets} bullets in {chambers} chambers."
                ))
                .build();
            if let Err(err) = bot.send_message(&send_message_param).await {
                error!("Failed to send message: {err}");
            }

            // Fire the roulette again
            let Some(result) = roulette.fire() else {
                error!("Failed to fire roulette!");
                return Some("You're lucky that the gun got jammed.".to_string());
            };
            result
        }
    };

    // Apply action and return the message
    let name = sender.username.as_deref();
    let name = name.unwrap_or(&sender.first_name);
    if result {
        // Restrict the user for a certain period
        let until = roulette_config.random_mute_until();
        let restrict_param = RestrictChatMemberParams::builder()
            .chat_id(chat.id)
            .user_id(sender.id)
            .permissions(RESTRICTED_PERM)
            .until_date(until)
            .build();
        match bot.restrict_chat_member(&restrict_param).await {
            Ok(_) => {
                info!("Restricted user {} in group ID {}", name, chat.id);
            }
            Err(err) => {
                error!("Failed to restrict user {}: {err}", name);
                return None;
            }
        };
        Some(format!("{name} is shot."))
    } else {
        Some(format!("{name} is safe and sound."))
    }
}
