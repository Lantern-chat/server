use super::*;

pub fn add_id_lock_cleanup_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(30),
        move |state, _| async move {
            log::trace!("Cleaning up ID locks");
            state.id_lock.cleanup().await;
        },
    )))
}
