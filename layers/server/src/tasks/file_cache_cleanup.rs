use super::*;

pub fn add_file_cache_cleanup_task(state: &ServerState, runner: &TaskRunner) {
    #[cfg(debug_assertions)]
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 1);

    #[cfg(not(debug_assertions))]
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 5);

    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        CLEANUP_INTERVAL,
        |state, _| async move {
            log::trace!("Cleaning up file cache");
            state
                .file_cache
                .cleanup(Duration::from_secs(state.config().web.file_cache_secs))
                .await;
        },
    )))
}
