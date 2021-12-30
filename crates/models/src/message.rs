use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageCreateForm {
    pub content: SmolStr,

    #[serde(default, skip_serializing_if = "crate::is_false")]
    pub tts: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<File>,
}

bitflags::bitflags! {
    pub struct MessageFlags: i16 {
        const DELETED           = 1 << 0;
        const MENTIONS_EVERYONE = 1 << 1;
        const MENTIONS_HERE     = 1 << 2;
        const PINNED            = 1 << 3;
        const TTS               = 1 << 4;
        const SUPRESS_EMBEDS    = 1 << 5;
    }
}

serde_shims::impl_serde_for_bitflags!(MessageFlags);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Snowflake,
    pub room_id: Snowflake,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party_id: Option<Snowflake>,

    pub author: User,

    /// Partial PartyMember
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member: Option<PartyMember>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<Snowflake>,

    pub created_at: Timestamp,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<Timestamp>,

    pub content: SmolStr,

    pub flags: MessageFlags,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_mentions: Vec<Snowflake>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub role_mentions: Vec<Snowflake>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub room_mentions: Vec<Snowflake>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reactions: Vec<Reaction>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionShorthand {
    pub emote: Snowflake,
    pub own: bool,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionFull {
    pub emote: Emote,
    pub users: Vec<Snowflake>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Reaction {
    Shorthand(ReactionShorthand),
    Full(ReactionFull),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    #[serde(flatten)]
    pub file: File,
}
