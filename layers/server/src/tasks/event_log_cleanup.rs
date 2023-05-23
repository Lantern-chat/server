use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::Error;

use super::*;

pub fn add_event_log_cleanup_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(60),
        |state, _| async move {
            log::trace!("Cleaning up event_log");

            let task = async {
                let db = state.db.write.get().await?;

                let last_event = state.gateway.last_events[1].load(Ordering::SeqCst);

                db.execute2(schema::sql! {
                    DELETE FROM EventLog WHERE EventLog.Counter <= #{&last_event as EventLog::Counter}
                })
                .await?;

                // take most recent event and put it into the second element
                state.gateway.last_events[1]
                    .store(state.gateway.last_events[0].load(Ordering::SeqCst), Ordering::SeqCst);

                Ok::<(), Error>(())
            };

            if let Err(e) = task.await {
                log::error!("Error cleaning event_log: {e}");
            }
        },
    )))
}
