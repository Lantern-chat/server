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

impl User {
    pub fn from_row(row: &Row) -> Self {
        User {
            id: row.get(0),
            username: row.get(1),
            nickname: row.get(2),
            blurb: row.get(3),
            avatar_id: row.get(4),
            preferences: row.get(5),
        }
    }
}
