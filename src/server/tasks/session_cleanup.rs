use std::time::Duration;
use std::{sync::Arc, time::SystemTime};

use crate::server::ServerState;

pub async fn cleanup_sessions(state: ServerState) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 5));

    while state.is_alive() {
        log::trace!("Cleaning up old user sessions");

        let _ = interval.tick().await;

        let res = state
            .db
            .execute_cached(
                || "DELETE FROM lantern.sessions WHERE expires < $1",
                &[&SystemTime::now()],
            )
            .await;

        if let Err(e) = res {
            log::error!("{}", e);
        }
    }
}
