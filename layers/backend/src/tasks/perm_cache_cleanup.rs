use super::*;

pub fn perm_cache_cleanup(state: State, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        Duration::from_secs(5),
        move |_, _| async {
            log::trace!("Cleaning up permission cache");
            state.perm_cache.cleanup().await;
        },
    )))
}
