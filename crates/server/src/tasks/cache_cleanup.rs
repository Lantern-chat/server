use std::time::Duration;

use crate::ServerState;

pub async fn cache_cleanup(state: ServerState) {
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(5);

    let mut interval = tokio::time::interval(CLEANUP_INTERVAL);

    while state.is_alive() {
        let now = tokio::select! {
            biased;
            now = interval.tick() => { now.into_std() },
            _ = state.notify_shutdown.notified() => { break; }
        };

        log::trace!("Cleaning up item cache");

        state
            .item_cache
            .retain(|_, (t, _)| now.duration_since(*t) < CLEANUP_INTERVAL)
            .await;
    }
}
