use task_runner::{AsyncFnTask, IntervalFnTask, RetryTask, TaskRunner};

use crate::ServerState;

pub fn add_tasks(state: &ServerState, runner: &TaskRunner) {
    cn_cleanup::add_cleanup_connections_task(state.clone(), runner);
    file_cache_cleanup::add_file_cache_cleanup_task(state.clone(), runner);
    http_server::add_http_server_task(state.clone(), runner);
    rl_cleanup::add_cleanup_ratelimit_task(state.clone(), runner);
}

use std::time::Duration;

mod cn_cleanup;
mod file_cache_cleanup;
mod http_server;
mod rl_cleanup;
