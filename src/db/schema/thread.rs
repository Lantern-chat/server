use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: Snowflake,
    pub parent: Snowflake,
}

impl Thread {}
