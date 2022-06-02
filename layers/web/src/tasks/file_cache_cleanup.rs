use std::time::Duration;

use crate::ServerState;

pub async fn file_cache_cleanup(state: ServerState) {
    #[cfg(debug_assertions)]
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 1);

    #[cfg(not(debug_assertions))]
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 5);

    let mut interval = tokio::time::interval(CLEANUP_INTERVAL);

    while state.is_alive() {
        tokio::select! {
            biased;
            _ = interval.tick() => {},
            _ = state.notify_shutdown.notified() => { break; }
        };

        log::trace!("Cleaning up file cache");
        state.file_cache.cleanup().await;
    }
}
