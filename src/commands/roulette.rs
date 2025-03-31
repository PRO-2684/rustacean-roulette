use super::{Command, Roulette, RouletteConfig};
use frankenstein::{
    AsyncTelegramApi,
    client_reqwest::Bot,
    methods::{GetChatMemberParams, RestrictChatMemberParams, SendMessageParams},
    types::{ChatMember, ChatPermissions, Message},
};
use log::{error, info};
use tokio::sync::Mutex;

/// Joins the roulette game.
pub struct RouletteCommand;

impl Command for RouletteCommand {
    const TRIGGER: &'static str = "roulette";
    const HELP: &'static str = "Joins the roulette game.";
    async fn execute(
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
}
