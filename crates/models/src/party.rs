use std::ops::Deref;

use super::*;

bitflags::bitflags! {
    pub struct SecurityFlags: i8 {
        /// Must have a verified email address
        const EMAIL         = 1 << 0;
        /// Must have a verified phone number
        const PHONE         = 1 << 1;
        /// Must be a Lantern user for longer than 5 minutes
        const NEW_USER      = 1 << 2;
        /// Must be a member of the server for longer than 10 minutes
        const NEW_MEMBER    = 1 << 3;
        /// Must have MFA enabled
        const MFA_ENABLED   = 1 << 4;
    }
}

serde_shims::impl_serde_for_bitflags!(SecurityFlags);

impl Default for SecurityFlags {
    fn default() -> Self {
        SecurityFlags::empty()
    }
}

//#[derive(Debug, Clone, Serialize, Deserialize)]
//#[serde(untagged)]
//pub enum UnvailableParty {
//    Available(Party),
//    Unavailable { id: Snowflake, unavailable: bool },
//}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    #[serde(flatten)]
    pub partial: PartialParty,

    /// Id of owner user
    pub owner: Snowflake,

    pub security: SecurityFlags,

    pub roles: Vec<Role>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emotes: Vec<Emote>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_id: Option<Snowflake>,

    pub sort_order: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialParty {
    pub id: Snowflake,

    /// Party name
    pub name: String,

    /// Discription of the party, if publicly listed
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Deref for Party {
    type Target = PartialParty;

    fn deref(&self) -> &Self::Target {
        &self.partial
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMember {
    /// Global user information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,

    /// Per-party nickname
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,

    // /// Per-party status
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub status: Option<String>,
    /// Per-party biography
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub bio: Option<String>,

    /// Per-party avatar?
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub avatar_id: Option<Snowflake>,

    /// List of Role id snowflakes
    #[serde(default, skip_serializing_if = "is_none_or_empty")]
    pub roles: Option<Vec<Snowflake>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presence: Option<UserPresence>,
}