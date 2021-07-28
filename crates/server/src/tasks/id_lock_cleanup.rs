use std::time::Duration;

use crate::ServerState;

pub async fn id_lock_cleanup(state: ServerState) {
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(30);

    let mut interval = tokio::time::interval(CLEANUP_INTERVAL);

    while state.is_alive() {
        tokio::select! {
            biased;
            _ = interval.tick() => {},
            _ = state.notify_shutdown.notified() => { break; }
        };

        log::trace!("Cleaning up ID locks");
        state.id_lock.cleanup().await;
    }
}
