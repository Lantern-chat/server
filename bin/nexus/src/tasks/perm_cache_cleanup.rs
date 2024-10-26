use super::*;

pub fn add_perm_cache_cleanup(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(5),
        |state, _| async move {
            log::trace!("Cleaning up permission cache");
            state.perm_cache.cleanup().await;
        },
    )))
}
