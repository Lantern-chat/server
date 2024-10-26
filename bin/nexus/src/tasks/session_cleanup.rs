use timestamp::Timestamp;

use super::*;

pub fn add_session_cleanup_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(60 * 5),
        |state, _| async move {
            debug_assert!(state.config().local.node.is_user_nexus());

            log::trace!("Cleaning up old user sessions");

            let now = Timestamp::now_utc();

            let db = match state.db.write.get().await {
                Ok(db) => db,
                Err(e) => {
                    log::error!("Error getting database connection for session cleanup task: {e}");
                    return;
                }
            };

            let running_cleanup = db.execute2(schema::sql! {
                DELETE FROM Sessions WHERE Sessions.Expires < #{&now as Sessions::Expires}
            });

            if let Err(e) = running_cleanup.await {
                log::error!("Error during session cleanup: {e}");
            }
        },
    )))
}
