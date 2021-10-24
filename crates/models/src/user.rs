use super::*;

bitflags::bitflags! {
    /// NOTE: Remember to clear flag caches when they change
    pub struct UserFlags: i16 {
        //const SYSTEM                = 1 << 2;
        //const BOT                   = 1 << 3;
        //const STAFF                 = 1 << 4;

        const DELETED               = 1 << 0;
        const VERIFIED              = 1 << 1;
        const MFA_ENABLED           = 1 << 2;
        const NEEDS_PASSWORD_RESET  = 1 << 3;

        const RESERVED_1            = 1 << 4;
        const RESERVED_2            = 1 << 5;

        // 3-bit integer
        const ELEVATION_1           = 1 << 6;
        const ELEVATION_2           = 1 << 7;
        const ELEVATION_3           = 1 << 8;

        // 3-bit integer
        const PREMIUM_1             = 1 << 9;
        const PREMIUM_2             = 1 << 10;
        const PREMIUM_3             = 1 << 11;

        const RESERVED_3            = 1 << 12;

        // 2-bit integer
        const EXTRA_STORAGE_1       = 1 << 13;
        const EXTRA_STORAGE_2       = 1 << 14;

        const RESERVED_4            = 1 << 15;

        const RESERVED = Self::RESERVED_1.bits | Self::RESERVED_2.bits | Self::RESERVED_3.bits | Self::RESERVED_4.bits;

        /// Always strip these from public responses
        const PRIVATE_FLAGS = Self::VERIFIED.bits | Self::MFA_ENABLED.bits | Self::DELETED.bits | Self::NEEDS_PASSWORD_RESET.bits | Self::EXTRA_STORAGE.bits | Self::RESERVED.bits;

        /// elevation level integer
        const ELEVATION     = Self::ELEVATION_1.bits | Self::ELEVATION_2.bits | Self::ELEVATION_3.bits;

        /// premium level integer
        const PREMIUM       = Self::PREMIUM_1.bits | Self::PREMIUM_2.bits | Self::PREMIUM_3.bits;

        /// extra storage level integer
        const EXTRA_STORAGE = Self::EXTRA_STORAGE_1.bits | Self::EXTRA_STORAGE_2.bits;
    }
}

serde_shims::impl_serde_for_bitflags!(UserFlags);

#[repr(u8)]
pub enum ElevationLevel {
    None = 0,
    Bot = 1,

    Reserved = 2,

    Staff = 3,
    System = 4,
}

impl UserFlags {
    /// Cleanup any private flags for public responses
    #[inline]
    pub fn publicize(mut self) -> Self {
        self.remove(Self::PRIVATE_FLAGS);
        self
    }

    pub fn elevation(self) -> ElevationLevel {
        match (self & Self::ELEVATION).bits() >> 6 {
            1 => ElevationLevel::Bot,
            3 => ElevationLevel::Staff,
            4 => ElevationLevel::System,
            _ => ElevationLevel::None,
        }
    }

    pub fn premium_level(self) -> u8 {
        ((self & Self::PREMIUM).bits() >> 9) as u8
    }

    pub fn extra_storage_tier(self) -> u8 {
        ((self & Self::EXTRA_STORAGE).bits() >> 13) as u8
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Snowflake,
    pub username: SmolStr,

    /// Unsigned 16-bit integer
    pub discriminator: i32,
    pub flags: UserFlags,
    pub avatar: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<SmolStr>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bio: Option<SmolStr>,

    /// Not present when user isn't self
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<SmolStr>,

    /// Not present when user isn't self
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferences: Option<UserPreferences>,
}

bitflags::bitflags! {
    pub struct FriendFlags: i16 {
        /// Pins the user to the top of their friendlist
        const FAVORITE = 1 << 0;
    }
}

serde_shims::impl_serde_for_bitflags!(FriendFlags);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<SmolStr>,
    pub flags: FriendFlags,
    pub user: User,
}
