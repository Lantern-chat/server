use super::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invite {
    pub id: Snowflake,
    pub code: String,
    pub party_id: Snowflake,
    pub inviter: Snowflake,
    pub description: String,
    pub expires: time::PrimitiveDateTime,
    pub uses: i8,
}
