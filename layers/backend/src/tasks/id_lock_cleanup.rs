use super::*;

pub fn add_id_lock_cleanup_task(state: State, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        Duration::from_secs(30),
        move |_, _| async {
            log::trace!("Cleaning up ID locks");
            state.id_lock.cleanup().await;
        },
    )))
}
