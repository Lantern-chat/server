use task_runner::{AsyncFnTask, IntervalFnTask, RetryTask, TaskRunner};

use crate::prelude::*;

pub fn add_tasks(state: &ServerState, runner: &TaskRunner) {
    gateway_event_cleanup::add_gateway_event_cleanup_task(state, runner);
}

use std::sync::atomic::Ordering;
use std::time::Duration;

mod gateway_event_cleanup;
