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

        match state.db.write.get().await {
            Ok(db) => {
                let res = db
                    .execute_cached(
                        || "DELETE FROM lantern.sessions WHERE expires < $1",
                        &[&SystemTime::now()],
                    )
                    .await;

                if let Err(e) = res {
                    log::error!("Error during session cleanup: {}", e);
                }
            }
            Err(e) => {
                log::error!(
                    "Unable to run session cleanup task due to error: {}\nTrying again in 1 second.",
                    e
                );
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
