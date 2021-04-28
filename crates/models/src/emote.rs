use super::*;

bitflags::bitflags! {
    pub struct EmoteFlags: u16 {
        const ANIMATED = 1 << 0;
        const STICKER  = 1 << 1;
    }
}

serde_shims::impl_serde_for_bitflags!(EmoteFlags);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Emote {
    Standard {
        name: char,
    },
    Custom {
        id: Snowflake,
        name: String,
        flags: EmoteFlags,
    },
}
