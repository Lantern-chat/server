use std::time::Duration;

use crate::ServerState;

pub async fn cleanup_connections(state: ServerState) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    while state.is_alive() {
        log::trace!("Cleaning up dead connections");

        let _now = interval.tick().await;

        // TODO: Cleanup dead connections by checking last-heartbeat
    }
}
