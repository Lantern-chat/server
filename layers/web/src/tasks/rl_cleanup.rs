use super::*;

pub fn add_cleanup_ratelimit_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(task_runner::interval_fn_task(
        state.clone(),
        Duration::from_secs(10),
        |now, state| async {
            log::trace!("Cleaning old rate-limit entries");
            state.rate_limit.cleanup_at(now.into_std()).await;
        },
    ))
}
