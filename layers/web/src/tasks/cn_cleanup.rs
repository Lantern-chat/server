use super::*;

pub fn add_cleanup_connections_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(task_runner::interval_fn_task(
        state.clone(),
        Duration::from_secs(5),
        |_, state| async {
            log::trace!("Cleaning up dead connections");

            // TODO: Cleanup dead connections by checking last-heartbeat
        },
    ))
}
