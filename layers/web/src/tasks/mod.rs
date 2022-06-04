use task_runner::TaskRunner;

use crate::ServerState;

pub fn add_tasks(state: &ServerState, runner: &TaskRunner) {
    cn_cleanup::add_cleanup_connections_task(state, runner);
    file_cache_cleanup::add_file_cache_cleanup_task(state, runner);
    http_server::add_http_server_task(state, runner);
    rl_cleanup::add_cleanup_ratelimit_task(state, runner);
}

use std::time::Duration;

mod cn_cleanup;
mod file_cache_cleanup;
mod http_server;
mod rl_cleanup;
