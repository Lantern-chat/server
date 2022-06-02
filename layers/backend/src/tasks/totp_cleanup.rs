use std::time::Duration;

use crate::ServerState;

pub async fn totp_cleanup(state: ServerState) {
    const LIMIT: Duration = Duration::from_secs(60 * 10);

    let mut interval = tokio::time::interval(Duration::from_secs(15));

    while state.is_alive() {
        let now = tokio::select! {
            biased;
            now = interval.tick() => { now.into_std() },
            _ = state.notify_shutdown.notified() => { break; }
        };

        log::trace!("Cleaning up totp tokens");

        state
            .totp_tokens
            .map
            .retain(|_, (t, _)| now.duration_since(*t) < LIMIT)
            .await;
    }
}
