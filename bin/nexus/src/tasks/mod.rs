pub use task_runner::{AsyncFnTask, IntervalFnTask, RetryTask, TaskRunner};

use std::{sync::atomic::Ordering, time::Duration};

type Alive = tokio::sync::watch::Receiver<bool>;

use crate::prelude::*;

pub fn add_tasks(state: &ServerState, runner: &TaskRunner) {
    let config = state.config();

    rpc_server::add_rpc_server_task(state, runner);
    gateway_event_cleanup::add_gateway_event_cleanup_task(state, runner);
    perm_cache_cleanup::add_perm_cache_cleanup(state, runner);

    if config.local.node.is_user_nexus() {
        mfa_cleanup::add_mfa_cleanup_tasks(state, runner);
        session_cleanup::add_session_cleanup_task(state, runner);
    }

    crate::gateway::task::listen::add_gateway_listener(state.clone(), runner);
    crate::gateway::task::process::add_gateway_processor(state.clone(), runner);
}

mod gateway_event_cleanup;
mod mfa_cleanup;
mod perm_cache_cleanup;
mod rpc_server;
mod session_cleanup;
