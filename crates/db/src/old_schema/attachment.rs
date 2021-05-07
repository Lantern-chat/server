use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Snowflake,
    pub file_id: Snowflake,
    pub message_id: Snowflake,
}

impl Attachment {}
