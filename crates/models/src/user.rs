use super::*;

bitflags::bitflags! {
    pub struct UserFlags: u16 {
        const VERIFIED    = 1 << 0;
        const MFA_ENABLED = 1 << 1;
        const SYSTEM      = 1 << 2;
        const BOT         = 1 << 3;
        const STAFF       = 1 << 4;
    }
}

serde_shims::impl_serde_for_bitflags!(UserFlags);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    pub descriminator: i16,
    pub flags: UserFlags,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub nickname: Option<String>,
    pub blurb: Option<String>,
    pub avatar_id: Option<Snowflake>,
    pub preferences: Option<UserPreferences>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub locale: String,
}
