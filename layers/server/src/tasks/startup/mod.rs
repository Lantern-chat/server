use crate::ServerState;
use task_runner::{AsyncFnTask, TaskRunner};

// structured this way to allow for batching of future tasks
pub async fn run_startup_tasks(state: &ServerState) {
    let runner = TaskRunner::default();
    insert_emojis::add_emoji_insert_task(state.clone(), &runner);
    runner.wait().await.unwrap();

    let runner = TaskRunner::default();
    refresh_emojis::add_emoji_refresh_task(state.clone(), &runner);
    runner.wait().await.unwrap();
}

mod insert_emojis;
mod refresh_emojis;
