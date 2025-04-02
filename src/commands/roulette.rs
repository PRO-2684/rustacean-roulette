use super::{Command, Roulette};
use crate::constants::RESTRICTED_PERM;
use frankenstein::{
    AsyncTelegramApi,
    client_reqwest::Bot,
    methods::{GetChatMemberParams, RestrictChatMemberParams},
    types::{ChatMember, Message},
};
use log::{error, info};
use tokio::sync::Mutex;

/// Joins the roulette game.
pub struct RouletteCommand;

impl Command for RouletteCommand {
    const TRIGGER: &'static str = "roulette";
    const HELP: &'static str = "Joins the roulette game.";
    async fn execute(bot: &Bot, msg: Message, roulette: &Mutex<Roulette>) -> Option<String> {
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
                // This should never happen, but just in case
                error!("Failed to fire the roulette: {roulette:?}");
                // Reload the gun
                roulette.restart();
                let (bullets, chambers) = roulette.info();
                return Some(format!(
                    "You're lucky that the gun got jammed. The gun has been reloaded, with {bullets} bullets in {chambers} chambers."
                ));
            }
        };

        // Reload the gun if empty
        let reload_tip = if roulette.peek().0 == 0 {
            roulette.restart();
            let (bullets, chambers) = roulette.info();
            format!(" The gun has been reloaded, with {bullets} bullets in {chambers} chambers.")
        } else {
            String::new()
        };

        // Apply action and return the message
        let name = sender.username.as_deref();
        let name = name.unwrap_or(&sender.first_name);
        if result {
            // Restrict the user for a certain period
            let (duration, until) = roulette.random_mute_until();
            let restrict_param = RestrictChatMemberParams::builder()
                .chat_id(chat.id)
                .user_id(sender.id)
                .permissions(RESTRICTED_PERM)
                .until_date(until)
                .build();
            match bot.restrict_chat_member(&restrict_param).await {
                Ok(_) => {
                    info!(
                        "Restricted user {name} for {duration}s in group <{}>",
                        chat.id
                    );
                }
                Err(err) => {
                    error!("Failed to restrict user {name}: {err}");
                    return None;
                }
            };
            Some(format!("Bang! {name} was shot and muted for {duration}s.",) + &reload_tip)
        } else {
            Some(format!("Click! {name} is safe and sound.",) + &reload_tip)
        }
    }
}
