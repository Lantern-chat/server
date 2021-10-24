use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub code: SmolStr,
    pub party: PartialParty,
    pub inviter: Snowflake,

    #[serde(skip_serializing_if = "SmolStr::is_empty")]
    pub description: SmolStr,

    pub expires: Option<Timestamp>,

    /// Number of remaining uses this invite has left.
    ///
    /// Only users with the `MANAGE_INVITES` permission can see this.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<u16>,
}
