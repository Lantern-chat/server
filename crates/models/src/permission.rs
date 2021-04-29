use super::*;

bitflags::bitflags! {
    /// Permissions that make sense with party-wide roles
    pub struct PartyPermissions: i16 {
        const CREATE_INVITE     = 1 << 0;
        const KICK_MEMBERS      = 1 << 1;
        const BAN_MEMBERS       = 1 << 2;
        const ADMINISTRATOR     = 1 << 3;
        const VIEW_AUDIT_LOG    = 1 << 4;
        const VIEW_STATISTICS   = 1 << 5;
        const MANAGE_PARTY      = 1 << 6;
        const MANAGE_ROOMS      = 1 << 7;
        const MANAGE_NICKNAMES  = 1 << 8;
        const MANAGE_ROLES      = 1 << 9;
        const MANAGE_WEBHOOKS   = 1 << 10;
        const MANAGE_EMOJIS     = 1 << 11;
        const MOVE_MEMBERS      = 1 << 12;
        const CHANGE_NICKNAME   = 1 << 13;
    }
}

bitflags::bitflags! {
    /// Permissions that make sense with per-room overrides
    pub struct RoomPermissions: i16 {
        const VIEW_ROOM             = 1 << 0;
        const READ_MESSAGES         = 1 << 1;
        const SEND_MESSAGES         = 1 << 2;
        const MANAGE_MESSAGES       = 1 << 3;
        const MUTE_MEMBERS          = 1 << 4;
        const DEAFEN_MEMBERS        = 1 << 5;
        const MENTION_EVERYONE      = 1 << 6;
        const USE_EXTERNAL_EMOTES   = 1 << 7;
        const ADD_REACTIONS         = 1 << 8;
        const EMBED_LINKS           = 1 << 9;
        const ATTACH_FILES          = 1 << 10;
        const USE_SLASH_COMMANDS    = 1 << 11;
        const SEND_TTS_MESSAGES     = 1 << 12;
    }
}

bitflags::bitflags! {
    /// Permissions that make sense on stream rooms
    pub struct StreamPermissions: i16 {
        /// Allows a user to broadcast a stream to this room
        const STREAM            = 1 << 0;
        /// Allows a user to connect and watch/listen to streams in a room
        const CONNECT           = 1 << 1;
        /// Allows a user to speak in a room without broadcasting a stream
        const SPEAK             = 1 << 2;
        /// Allows a user to acquire priority speaker
        const PRIORITY_SPEAKER  = 1 << 3;
    }
}

serde_shims::impl_serde_for_bitflags!(PartyPermissions);
serde_shims::impl_serde_for_bitflags!(RoomPermissions);
serde_shims::impl_serde_for_bitflags!(StreamPermissions);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Permission {
    pub party: PartyPermissions,
    pub room: RoomPermissions,
    pub stream: StreamPermissions,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Overwrite {
    /// Role or user ID
    ///
    /// If it doesn't exist in the role list, then it's a user, simple as that
    pub id: Snowflake,
    pub allow: Permission,
    pub deny: Permission,
}

impl Permission {
    #[inline]
    pub const fn pack(self) -> u64 {
        let low = self.party.bits() as u64;
        let mid = self.room.bits() as u64;
        let high = self.stream.bits() as u64;

        // NOTE: These must be updated if the field size is changed
        low | (mid << 16) | (high << 32)
    }

    #[inline]
    pub const fn unpack(bits: u64) -> Self {
        Permission {
            party: PartyPermissions::from_bits_truncate(bits as i16),
            room: RoomPermissions::from_bits_truncate((bits >> 16) as i16),
            stream: StreamPermissions::from_bits_truncate((bits >> 32) as i16),
        }
    }
}
