use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub code: String,
    pub party: PartialParty,
    pub inviter: Snowflake,
    pub description: String,
}
