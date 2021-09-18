use std::num::NonZeroU32;

use super::*;

bitflags::bitflags! {
    pub struct RoomFlags: i16 {
        const TEXT    = 1 << 0;
        const DIRECT  = 1 << 1;
        const VOICE   = 1 << 2;
        const GROUP   = 1 << 3;
        const NSFW    = 1 << 4;
        const DEFAULT = 1 << 5;
    }
}

serde_shims::impl_serde_for_bitflags!(RoomFlags);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: Snowflake,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party_id: Option<Snowflake>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<SmolStr>,

    pub name: SmolStr,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<SmolStr>,

    /// Sort order
    pub sort_order: i16,

    pub flags: RoomFlags,

    /// Slow-mode rate limit, in seconds
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<NonZeroU32>,

    /// Parent room ID for categories
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Snowflake>,

    /// Permission overwrites for this room
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overwrites: Vec<Overwrite>,
    // /// Direct/Group Message Users
    // #[serde(default, skip_serializing_if = "Vec::is_empty")]
    // pub recipients: Vec<User>,
}
