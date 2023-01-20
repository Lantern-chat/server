use super::*;

/// Events intended for internal-use only, does not require encoding or compression,
/// and are typically sent directly to user connections
#[derive(Debug)]
pub enum InternalEvent {
    BulkUserBlockedRefresh { blocked: Vec<Snowflake> },
    UserBlockedAdd { user_id: Snowflake },
    UserBlockedRemove { user_id: Snowflake },
}
