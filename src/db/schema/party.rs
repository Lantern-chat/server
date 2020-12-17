use crate::db::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub name: String,
}
