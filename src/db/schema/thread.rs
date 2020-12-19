use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: Snowflake,
    pub parent: Snowflake,
}

impl Thread {
    pub fn from_row(row: &Row) -> Thread {
        Thread {
            id: row.get(0),
            parent: row.get(1),
        }
    }
}
