use std::time::Duration;

use crate::ServerState;

pub async fn perm_cache_cleanup(state: ServerState) {
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(5);

    let mut interval = tokio::time::interval(CLEANUP_INTERVAL);

    while state.is_alive() {
        tokio::select! {
            biased;
            _ = interval.tick() => {},
            _ = state.notify_shutdown.notified() => { break; }
        };

        log::trace!("Cleaning up permission cache");
        state.perm_cache.cleanup().await;
    }
}
