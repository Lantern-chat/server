use task_runner::{AsyncFnTask, IntervalFnTask, RetryTask, TaskRunner};

use crate::ServerState;

pub fn add_tasks(state: &ServerState, runner: &TaskRunner) {
    cn_cleanup::add_cleanup_connections_task(state, runner);
    file_cache_cleanup::add_file_cache_cleanup_task(state, runner);
    http_server::add_http_server_task(state, runner);
    rl_cleanup::add_cleanup_ratelimit_task(state, runner);
    event_log_cleanup::add_event_log_cleanup_task(state, runner);
    id_lock_cleanup::add_id_lock_cleanup_task(state, runner);
    perm_cache_cleanup::perm_cache_cleanup(state, runner);
    record_metrics::add_record_metrics_task(state, runner);
    session_cleanup::add_cleanup_sessions_task(state, runner);
}

use std::time::Duration;

mod cn_cleanup;
mod event_log_cleanup;
mod file_cache_cleanup;
mod http_server;
mod id_lock_cleanup;
mod perm_cache_cleanup;
mod record_metrics;
mod rl_cleanup;
mod session_cleanup;
