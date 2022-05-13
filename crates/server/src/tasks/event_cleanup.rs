use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::ctrl::Error;
use crate::ServerState;

pub async fn cleanup_events(state: ServerState) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    while state.is_alive() {
        let _now = tokio::select! {
            biased;
            now = interval.tick() => { now },
            _ = state.notify_shutdown.notified() => { break; }
        };

        log::trace!("Cleaning up event_log");

        let task = async {
            let db = state.db.write.get().await?;

            let last_event = state.last_events[1].load(Ordering::SeqCst);

            db.execute_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    Query::delete()
                        .from::<EventLog>()
                        .and_where(EventLog::Counter.less_than_equal(Var::of(EventLog::Counter)))
                },
                &[&last_event],
            )
            .await?;

            // take most recent event and put it into the second element
            state.last_events[1].store(state.last_events[0].load(Ordering::SeqCst), Ordering::SeqCst);

            Ok::<(), Error>(())
        };

        if let Err(e) = task.await {
            log::error!("Error cleaning event_log: {e}");
        }
    }
}
