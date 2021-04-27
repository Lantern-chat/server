use super::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub id: Snowflake,
    pub party_id: Snowflake,
    pub invitee: Snowflake,
    pub code: String,
    pub description: String,
    pub expires: (),
    pub uses: i8,
}
