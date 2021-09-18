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
    pub party_id: Snowflake,
    pub file: Snowflake,
    pub name: SmolStr,
    pub flags: EmoteFlags,
    pub aspect_ratio: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Emote {
    Standard { name: char },
    Custom(CustomEmote),
}
