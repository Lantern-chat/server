use super::*;

pub fn add_id_lock_cleanup_task(state: &State, runner: &TaskRunner) {
    runner.add(task_runner::interval_fn_task(
        state.clone(),
        Duration::from_secs(30),
        |_t, state| async {
            log::trace!("Cleaning up ID locks");
            state.id_lock.cleanup().await;
        },
    ))
}
