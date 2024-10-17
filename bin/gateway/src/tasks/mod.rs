pub use task_runner::{AsyncFnTask, IntervalFnTask, RetryTask, TaskRunner};

use std::{sync::atomic::Ordering, time::Duration};

type Alive = tokio::sync::watch::Receiver<bool>;

use crate::prelude::*;

pub fn add_tasks(state: &GatewayServerState, runner: &TaskRunner) {
    http_server::add_http_server_task(state, runner);
    https_server::add_https_server_task(state, runner);
}

pub mod http_server;
pub mod https_server;
