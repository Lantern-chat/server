use std::time::SystemTime;

use super::*;

pub fn add_cleanup_sessions_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(60 * 5),
        |state, _| async move {
            log::trace!("Cleaning up old user sessions");

            let now = SystemTime::now();

            #[rustfmt::skip]
            let db_task = async {
                match state.db.write.get().await {
                    Ok(db) => {
                        if let Err(e) = db.execute2(schema::sql! {
                            DELETE FROM Sessions WHERE Sessions.Expires < #{&now as Sessions::Expires}
                        }).await {
                            log::error!("Error during session cleanup: {e}");
                        }
                    }
                    Err(e) => log::error!("Database connection error during session cleanup: {e}"),
                }
            };

            tokio::join! {
                state.session_cache.cleanup(now),
                db_task,
            };
        },
    )))
}
