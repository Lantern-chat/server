use crate::ServerState;
use task_runner::{AsyncFnTask, IntervalFnTask, RetryTask, TaskRunner};

// structured this way to allow for batching of future tasks
pub async fn run_startup_tasks(state: &ServerState) {
    let runner = TaskRunner::new();
    insert_emojis::add_emoji_insert_task(state.clone(), &runner);
    runner.wait().await.unwrap();

    let runner = TaskRunner::new();
    refresh_emojis::add_emoji_refresh_task(state.clone(), &runner);
    runner.wait().await.unwrap();
}

mod insert_emojis;
mod refresh_emojis;
