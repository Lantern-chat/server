use task_runner::{AsyncFnTask, IntervalFnTask, RetryTask, TaskRunner};

use crate::ServerState;

pub fn add_tasks(state: &ServerState, runner: &TaskRunner) {
    clean_presence::add_clean_presence_task(state.clone(), runner);

    cn_cleanup::add_cleanup_connections_task(state, runner);
    file_cache_cleanup::add_file_cache_cleanup_task(state, runner);
    http_server::add_http_server_task(state, runner);
    rl_cleanup::add_cleanup_ratelimit_task(state, runner);
    event_log_cleanup::add_event_log_cleanup_task(state, runner);
    id_lock_cleanup::add_id_lock_cleanup_task(state, runner);
    mfa_cleanup::add_cleanup_mfa_task(state, runner);
    perm_cache_cleanup::perm_cache_cleanup(state, runner);
    record_metrics::add_record_metrics_task(state, runner);
    session_cleanup::add_cleanup_sessions_task(state, runner);
    file_cleanup::add_orphaned_file_cleanup_task(state, runner);

    crate::backend::gateway::task::listen::add_gateway_listener(state.clone(), runner);
    crate::backend::gateway::task::process::add_gateway_processor(state.clone(), runner);
}

use std::time::Duration;

mod clean_presence;
mod cn_cleanup;
mod event_log_cleanup;
mod file_cache_cleanup;
mod file_cleanup;
mod http_server;
mod id_lock_cleanup;
mod mfa_cleanup;
mod perm_cache_cleanup;
mod record_metrics;
mod rl_cleanup;
mod session_cleanup;

pub mod startup;
