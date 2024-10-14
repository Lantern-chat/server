pub use task_runner::{AsyncFnTask, IntervalFnTask, RetryTask, TaskRunner};

use std::{sync::atomic::Ordering, time::Duration};

type Alive = tokio::sync::watch::Receiver<bool>;

use crate::prelude::*;

pub fn add_tasks(state: &ServerState, runner: &TaskRunner) {
    rpc_server::add_rpc_server_task(state, runner);

    gateway_event_cleanup::add_gateway_event_cleanup_task(state, runner);
}

mod gateway_event_cleanup;
mod rpc_server;
