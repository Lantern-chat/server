use super::*;

#[derive(Debug)]
pub enum InternalEvent {
    BulkUserBlockedUpdate { blocked: Vec<Snowflake> },
}
