use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub code: SmolStr,
    pub party: PartialParty,
    pub inviter: Snowflake,
    pub description: SmolStr,

    /// Number of remaining uses this invite has left.
    ///
    /// Only users with the `MANAGE_INVITES` permission can see this.
    pub remaining: Option<u16>,
}
