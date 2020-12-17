use crate::db::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Snowflake,
    pub session_id: Snowflake,
    pub user_id: Snowflake,
    pub editor_id: Option<Snowflake>,
    pub room_id: Snowflake,
    pub thread_id: Option<Snowflake>,
    pub updated_at: (),
    pub content: String,
    pub pinned: bool,
}
