use super::*;

pub enum RoomType {}

bitflags::bitflags! {
    pub struct RoomFlags: i16 {
        const TEXT    = 1 << 0;
        const DIRECT  = 1 << 1;
        const VOICE   = 1 << 2;
        const GROUP   = 1 << 3;
        const NSFW    = 1 << 4;
    }
}

serde_shims::impl_serde_for_bitflags!(RoomFlags);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Snowflake,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party_id: Option<Snowflake>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pos: Option<u16>,

    pub flags: RoomFlags,
}
