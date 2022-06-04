use super::*;

pub fn add_file_cache_cleanup_task(state: &ServerState, runner: &TaskRunner) {
    #[cfg(debug_assertions)]
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 1);

    #[cfg(not(debug_assertions))]
    const CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 5);

    runner.add(task_runner::interval_fn_task(
        state.clone(),
        CLEANUP_INTERVAL,
        |_, state| async {
            log::trace!("Cleaning up file cache");
            state.file_cache.cleanup().await;
        },
    ))
}
