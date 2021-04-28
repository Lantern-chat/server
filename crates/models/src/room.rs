use super::*;

pub enum RoomType {}

bitflags::bitflags! {
    pub struct RoomFlags: i16 {
        const NSFW    = 1 << 0;
        const DIRECT  = 1 << 1;
    }
}

serde_shims::impl_serde_for_bitflags!(RoomFlags);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Snowflake,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub party_id: Option<Snowflake>,
}
