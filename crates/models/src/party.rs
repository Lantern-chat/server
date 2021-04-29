use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    #[serde(flatten)]
    pub partial: PartialParty,

    /// Id of owner user
    pub owner: Snowflake,

    pub roles: Vec<Role>,
    pub emotes: Vec<Emote>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartyMember {
    /// Global user information
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,

    /// Per-party nickname
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,

    /// Per-party status
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Per-party biography
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bio: Option<String>,

    /// Per-party avatar?
    // #[serde(default, skip_serializing_if = "Option::is_none")]
    // pub avatar_id: Option<Snowflake>,

    /// List of Role id snowflakes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<Snowflake>,
}
