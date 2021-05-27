use std::time::Duration;

use crate::ServerState;

pub async fn cleanup_ratelimits(state: ServerState) {
    let mut interval = tokio::time::interval(Duration::from_secs(10));

    loop {
        let now = tokio::select! {
            biased;
            now = interval.tick() => { now },
            _ = state.notify_shutdown.notified() => { break; }
        };

        log::trace!("Cleaning up rate-limits");
        state.rate_limit.cleanup_at(now.into_std()).await;
    }
}
