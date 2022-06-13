use std::time::SystemTime;

use super::*;

pub fn add_cleanup_sessions_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(60 * 5),
        |state, _, _| async move {
            log::trace!("Cleaning up old user sessions");

            let now = SystemTime::now();

            let db_task = async {
                match state.db.write.get().await {
                    Ok(db) => {
                        if let Err(e) = db.execute_cached_typed(|| query(), &[&now]).await {
                            log::error!("Error during session cleanup: {e}");
                        }
                    }
                    Err(e) => log::error!("Database connection error during session cleanup: {e}"),
                }
            };

            tokio::join! {
                state.session_cache.cleanup(now),
                db_task,
            };
        },
    )))
}

use thorn::*;

fn query() -> impl AnyQuery {
    use schema::*;

    Query::delete()
        .from::<Sessions>()
        .and_where(Sessions::Expires.less_than(Var::of(Sessions::Expires)))
}
