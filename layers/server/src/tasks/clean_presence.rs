use super::*;

pub fn add_clean_presence_task(state: ServerState, runner: &TaskRunner) {
    runner.add(AsyncFnTask::new(|_| async move {
        log::trace!("Cleaning up old presence values");

        let db = state.db.write.get().await.unwrap();

        db.execute2(schema::sql! { TRUNCATE ONLY UserPresence }).await.unwrap();
    }))
}
