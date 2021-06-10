use std::time::Duration;
use std::time::SystemTime;

use crate::ServerState;

pub async fn cleanup_sessions(state: ServerState) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 5));

    loop {
        tokio::select! {
            biased;
            _ = interval.tick() => {},
            _ = state.notify_shutdown.notified() => { break; }
        }

        log::trace!("Cleaning up old user sessions");

        let now = SystemTime::now();

        tokio::join! {
            state.session_cache.cleanup(now),
            async {
                match state.db.write.get().await {
                    Ok(db) => {
                        let res = db
                            .execute_cached(
                                || "DELETE FROM lantern.sessions WHERE expires < $1",
                                &[&now],
                            )
                            .await;

                        if let Err(e) = res {
                            log::error!("Error during session cleanup: {}", e);
                        }
                    }
                    Err(e) => log::error!("Database connection error during session cleanup: {}", e),
                }
            }
        };
    }
}
