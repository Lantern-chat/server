use super::*;

pub fn perm_cache_cleanup(state: &State, runner: &TaskRunner) {
    runner.add(task_runner::interval_fn_task(
        state.clone(),
        Duration::from_secs(5),
        |_, state| async {
            log::trace!("Cleaning up permission cache");
            state.perm_cache.cleanup().await;
        },
    ))
}
