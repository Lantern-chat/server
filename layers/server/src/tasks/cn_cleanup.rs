use super::*;

pub fn add_cleanup_connections_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(5),
        |state, _, _| async move {
            log::trace!("Cleaning up dead connections");

            // TODO: Cleanup dead connections by checking last-heartbeat
        },
    )))
}
