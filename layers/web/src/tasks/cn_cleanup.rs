use super::*;

pub fn add_cleanup_connections_task(state: ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        Duration::from_secs(5),
        move |_, _| async {
            log::trace!("Cleaning up dead connections");

            // TODO: Cleanup dead connections by checking last-heartbeat
        },
    )))
}
