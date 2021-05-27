use std::time::Duration;

use crate::ServerState;

pub async fn cleanup_connections(state: ServerState) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));

    while state.is_alive() {
        let _now = tokio::select! {
            biased;
            now = interval.tick() => { now },
            _ = state.notify_shutdown.notified() => { break; }
        };

        log::trace!("Cleaning up dead connections");

        // TODO: Cleanup dead connections by checking last-heartbeat
    }
}
