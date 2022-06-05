use task_runner::{IntervalFnTask, RetryTask, TaskRunner};

use crate::State;

pub fn add_tasks(state: &State, runner: &TaskRunner) {
    event_log_cleanup::add_event_log_cleanup_task(state.clone(), runner);
    id_lock_cleanup::add_id_lock_cleanup_task(state.clone(), runner);
    perm_cache_cleanup::perm_cache_cleanup(state.clone(), runner);
    record_metrics::add_record_metrics_task(state.clone(), runner);
    session_cleanup::add_cleanup_sessions_task(state.clone(), runner);
}

use std::time::Duration;

mod event_log_cleanup;
mod id_lock_cleanup;
mod perm_cache_cleanup;
mod record_metrics;
mod session_cleanup;
