use sdk::models::aliases::*;
use thin_vec::ThinVec;

/// Events intended for internal-use only, does not require encoding or compression,
/// and are typically sent directly to user connections
#[derive(Debug)]
pub enum InternalEvent {
    BulkUserBlockedRefresh { blocked: ThinVec<UserId> },
    UserBlockedAdd { user_id: UserId },
    UserBlockedRemove { user_id: UserId },
}
