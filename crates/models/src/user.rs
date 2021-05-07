use super::*;

bitflags::bitflags! {
    pub struct UserFlags: i16 {
        const VERIFIED    = 1 << 0;
        const MFA_ENABLED = 1 << 1;
        const SYSTEM      = 1 << 2;
        const BOT         = 1 << 3;
        const STAFF       = 1 << 4;
        const PREMIUM     = 1 << 5;

        /// Always strip these from public responses
        const PRIVATE_FLAGS = Self::VERIFIED.bits | Self::MFA_ENABLED.bits;
    }
}

serde_shims::impl_serde_for_bitflags!(UserFlags);

impl UserFlags {
    /// Cleanup any private flags for public responses
    #[inline]
    pub fn publicize(mut self) -> Self {
        self.remove(Self::PRIVATE_FLAGS);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    pub descriminator: i16,
    pub flags: UserFlags,
    pub avatar_id: Option<Snowflake>,

    /// Not present when user isn't self
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Not present when user isn't self
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferences: Option<UserPreferences>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub locale: String,
}
