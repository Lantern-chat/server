use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    pub nickname: Option<String>,
    pub blurb: Option<String>,
    pub avatar_id: Option<Snowflake>,
    pub preferences: String, // JSON
}

impl User {}
