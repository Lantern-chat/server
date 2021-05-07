use super::*;

#[derive(Debug, Clone)]
pub struct DirectMessage {
    pub user_a: Snowflake,
    pub user_b: Snowflake,
    pub channel_id: Snowflake,
}

#[derive(Debug, Clone)]
pub struct GroupMessage {
    pub id: Snowflake,
    pub channel_id: Snowflake,
}

#[derive(Debug, Clone)]
pub struct GroupMember {
    pub group_id: Snowflake,
    pub user_id: Snowflake,
}
