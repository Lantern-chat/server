use super::*;

pub fn add_emoji_refresh_task(state: ServerState, runner: &TaskRunner) {
    runner.add(AsyncFnTask::new(|_| async move {
        log::trace!("Refreshing emoji list");

        state.refresh_emojis().await.unwrap();
    }))
}
