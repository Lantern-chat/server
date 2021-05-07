use super::*;

#[derive(Debug, Clone)]
pub struct Thread {
    pub id: Snowflake,
    pub parent: Snowflake,
}

impl Thread {}
