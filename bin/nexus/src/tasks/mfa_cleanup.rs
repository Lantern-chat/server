use std::time::SystemTime;

use super::*;

pub fn add_mfa_cleanup_tasks(state: &ServerState, runner: &TaskRunner) {
    // Task for cleaning old MFA step counters
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(120),
        |state, _| async move {
            debug_assert!(state.config().local.node.is_user_nexus());

            // Get current time and divide it by the standard step size for TOTP MFA, being 30 seconds
            let now_step = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() / 30;

            // clean any steps before 60 seconds ago.
            let last_step = now_step - 2;

            log::trace!("Cleaning old MFA steps");
            state.mfa_last.retain_async(move |_, step| *step > last_step).await;
        },
    )));

    // Task for cleaning old pending MFA entries in the database
    #[rustfmt::skip]
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(60 * 30), // 30 minutes
        |state, _|  async move {
            debug_assert!(state.config().local.node.is_user_nexus());

            let Ok(db) = state.db.write.get().await else {
                log::error!("Error getting database connection for MFA Cleanup task");
                return;
            };

            if let Err(e) = db.execute2(schema::sql! {
                DELETE FROM MfaPending WHERE MfaPending.Expires <= now()
            }).await {
                log::error!("Error running MFA Cleanup task: {e}");
            }
        },
    )));
}
