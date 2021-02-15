use std::sync::Arc;
use std::time::Duration;

use crate::server::ServerState;

pub async fn cleanup_ratelimits(state: ServerState) {
    let mut interval = tokio::time::interval(Duration::from_secs(10));

    while state.is_alive() {
        log::trace!("Cleaning up rate-limits");

        let now = interval.tick().await;
        state.rate_limit.cleanup_at(now.into_std()).await;
    }
}
