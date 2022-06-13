use super::*;

pub fn add_cleanup_ratelimit_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(10),
         |state, now, _| async move {
            log::trace!("Cleaning old rate-limit entries");
            state.rate_limit.cleanup_at(now.into_std()).await;
        },
    )))
}
