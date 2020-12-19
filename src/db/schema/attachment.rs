use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Snowflake,
    pub file_id: Snowflake,
    pub message_id: Snowflake,
}

impl Attachment {
    pub fn from_row(row: &Row) -> Self {
        Attachment {
            id: row.get(0),
            file_id: row.get(1),
            message_id: row.get(2),
        }
    }
}
