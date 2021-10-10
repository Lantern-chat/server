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

        let db_task = async {
            match state.db.write.get().await {
                Ok(db) => {
                    if let Err(e) = db.execute_cached_typed(|| query(), &[&now]).await {
                        log::error!("Error during session cleanup: {}", e);
                    }
                }
                Err(e) => log::error!("Database connection error during session cleanup: {}", e),
            }
        };

        tokio::join! {
            state.session_cache.cleanup(now),
            db_task,
        };
    }
}

use thorn::*;

fn query() -> impl AnyQuery {
    use schema::*;

    Query::delete()
        .from::<Sessions>()
        .and_where(Sessions::Expires.less_than(Var::of(Sessions::Expires)))
}
