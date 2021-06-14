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
        const MANAGE_PERMS      = 1 << 14;

        const DEFAULT           = Self::CHANGE_NICKNAME.bits;
    }
}

bitflags::bitflags! {
    /// Permissions that make sense with per-room overrides
    pub struct RoomPermissions: i16 {
        const VIEW_ROOM             = 1 << 0;
        const READ_MESSAGES         = 1 << 1 | Self::VIEW_ROOM.bits;
        const SEND_MESSAGES         = 1 << 2 | Self::VIEW_ROOM.bits;
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

        const DEFAULT               = Self::VIEW_ROOM.bits |
                                      Self::READ_MESSAGES.bits |
                                      Self::SEND_MESSAGES.bits |
                                      Self::USE_EXTERNAL_EMOTES.bits |
                                      Self::ADD_REACTIONS.bits |
                                      Self::EMBED_LINKS.bits |
                                      Self::ATTACH_FILES.bits |
                                      Self::SEND_TTS_MESSAGES.bits;
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

        const DEFAULT           = Self::CONNECT.bits | Self::SPEAK.bits;
    }
}

serde_shims::impl_serde_for_bitflags!(PartyPermissions);
serde_shims::impl_serde_for_bitflags!(RoomPermissions);
serde_shims::impl_serde_for_bitflags!(StreamPermissions);

impl Default for PartyPermissions {
    fn default() -> Self {
        PartyPermissions::DEFAULT
    }
}

impl Default for RoomPermissions {
    fn default() -> Self {
        RoomPermissions::DEFAULT
    }
}

impl Default for StreamPermissions {
    fn default() -> Self {
        StreamPermissions::DEFAULT
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    #[serde(default, skip_serializing_if = "Permission::is_empty")]
    pub allow: Permission,
    #[serde(default, skip_serializing_if = "Permission::is_empty")]
    pub deny: Permission,
}

impl Permission {
    pub const fn empty() -> Self {
        Permission {
            party: PartyPermissions::empty(),
            room: RoomPermissions::empty(),
            stream: StreamPermissions::empty(),
        }
    }

    pub const ALL: Self = Permission {
        party: PartyPermissions::all(),
        room: RoomPermissions::all(),
        stream: StreamPermissions::all(),
    };

    pub const ADMIN: Self = Permission {
        party: PartyPermissions::ADMINISTRATOR,
        room: RoomPermissions::empty(),
        stream: StreamPermissions::empty(),
    };

    pub const VIEW_ROOM: Self = Permission {
        party: PartyPermissions::empty(),
        room: RoomPermissions::VIEW_ROOM,
        stream: StreamPermissions::empty(),
    };

    pub const PACKED_ALL: u64 = Self::ALL.pack();
    pub const PACKED_ADMIN: u64 = Self::ADMIN.pack();
    pub const PACKED_VIEW_ROOM: u64 = Self::VIEW_ROOM.pack();

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

    #[inline]
    pub fn remove(&mut self, other: Self) {
        self.party.remove(other.party);
        self.room.remove(other.room);
        self.stream.remove(other.stream);
    }

    #[inline]
    pub fn is_admin(&self) -> bool {
        self.party.contains(PartyPermissions::ADMINISTRATOR)
    }

    pub fn is_empty(&self) -> bool {
        self.party.is_empty() && self.room.is_empty() && self.stream.is_empty()
    }
}

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not};

impl Not for Permission {
    type Output = Self;

    #[inline(always)]
    fn not(self) -> Self {
        Permission {
            party: self.party.not(),
            room: self.room.not(),
            stream: self.stream.not(),
        }
    }
}

macro_rules! impl_bitwise {
    (@BINARY $($op_trait:ident::$op:ident),*) => {$(
        impl $op_trait for Permission {
            type Output = Permission;

            #[inline(always)]
            fn $op(self, rhs: Self) -> Self {
                Permission {
                    party: $op_trait::$op(self.party, rhs.party),
                    room: $op_trait::$op(self.room, rhs.room),
                    stream: $op_trait::$op(self.stream, rhs.stream),
                }
            }
        }
    )*};

    (@ASSIGN $($op_trait:ident::$op:ident),*) => {$(
        impl $op_trait for Permission {
            #[inline(always)]
            fn $op(&mut self, rhs: Self) {
                $op_trait::$op(&mut self.party, rhs.party);
                $op_trait::$op(&mut self.room, rhs.room);
                $op_trait::$op(&mut self.stream, rhs.stream);
            }
        }
    )*};
}

impl_bitwise!(@BINARY BitAnd::bitand, BitOr::bitor, BitXor::bitxor);
impl_bitwise!(@ASSIGN BitAndAssign::bitand_assign, BitOrAssign::bitor_assign, BitXorAssign::bitxor_assign);

impl Overwrite {
    #[inline]
    pub fn combine(&self, other: Self) -> Overwrite {
        Overwrite {
            id: self.id,
            allow: self.allow | other.allow,
            deny: self.deny | other.deny,
        }
    }

    #[inline]
    pub fn apply(&self, base: Permission) -> Permission {
        (base & !self.deny) | self.allow
    }
}
