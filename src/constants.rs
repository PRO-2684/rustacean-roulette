//! Constants for the bot.

use frankenstein::types::{ChatAdministratorRights, ChatPermissions};

/// Default number of chambers in the revolver.
pub fn chambers() -> usize {
    6
}

/// Default number of bullets in the revolver.
pub fn bullets() -> usize {
    2
}

/// Default probability of the gun getting jammed.
pub fn jam_probability() -> f64 {
    0.05 
}

/// Default minimum time to mute in seconds.
pub fn min_mute_time() -> u32 {
    60
}

/// Default maximum time to mute in seconds.
pub fn max_mute_time() -> u32 {
    600
}

/// Restricted permissions when someone got shot.
pub const RESTRICTED_PERM: ChatPermissions = ChatPermissions {
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

/// Recommended admin rights for the bot.
pub const RECOMMENDED_ADMIN_RIGHTS: ChatAdministratorRights = ChatAdministratorRights {
    is_anonymous: false,
    can_manage_chat: false,
    can_delete_messages: false,
    can_manage_video_chats: false,
    can_restrict_members: true, // Required
    can_promote_members: false,
    can_change_info: false,
    can_invite_users: false,
    can_post_messages: None,
    can_edit_messages: None,
    can_pin_messages: None,
    can_post_stories: None,
    can_edit_stories: None,
    can_delete_stories: None,
    can_manage_topics: None,
};
