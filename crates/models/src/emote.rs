use super::*;

bitflags::bitflags! {
    pub struct EmoteFlags: i16 {
        const ANIMATED = 1 << 0;
        const STICKER  = 1 << 1;
        const NSFW     = 1 << 2;
    }
}

serde_shims::impl_serde_for_bitflags!(EmoteFlags);

// TODO: Add inline preview?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEmote {
    pub id: Snowflake,
    pub file_id: Snowflake,
    pub name: String,
    pub flags: EmoteFlags,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Emote {
    Standard { name: char },
    Custom(CustomEmote),
}
