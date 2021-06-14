use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageCreateForm {
    pub content: String,

    #[serde(default, skip_serializing_if = "crate::is_false")]
    pub tts: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<Embed>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<File>,
}

bitflags::bitflags! {
    pub struct MessageFlags: i16 {
        const TTS               = 1 << 0;
        const MENTIONS_EVERYONE = 1 << 1;
        const MENTIONS_HERE     = 1 << 2;
        const SUPRESS_EMBEDS    = 1 << 3;
        const PINNED            = 1 << 4;
        const DELETED           = 1 << 5;
        const REMOVED           = 1 << 6;
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

    pub created_at: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited_at: Option<String>,

    pub content: String,

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
pub struct Reaction {
    pub emote: Emote,
    pub users: Vec<Snowflake>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Snowflake,
    pub filename: String,
    pub size: usize,

    #[serde(flatten)]
    pub embed: EmbedMediaAttributes,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embed {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts: Option<time::PrimitiveDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedMedia {
    #[serde(rename = "type")]
    pub kind: EmbedMediaKind,

    #[serde(flatten)]
    pub attr: EmbedMediaAttributes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbedMediaKind {
    Image,
    Video,
    Audio,
    Thumbnail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedMediaAttributes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
}
