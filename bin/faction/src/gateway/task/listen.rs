use db::{pg::AsyncMessage, pool::Object};
use futures::StreamExt;
use task_runner::{RetryAsyncFnTask, TaskRunner};

use crate::prelude::*;

pub fn add_gateway_listener(state: ServerState, runner: &TaskRunner) {
    runner.add(RetryAsyncFnTask::new(state, |mut alive, state| async move {
        // get new owned database connection
        let db = Object::take(state.db.read.get().await?);

        db.execute("LISTEN event_log", &[]).await?;

        let conn = db.take_connection().await;

        let mut stream = conn.stream.lock().await;

        loop {
            let event = tokio::select! {
                biased;
                event = stream.next() => event,
                _ = alive.changed() => break,
            };

            match event {
                Some(Ok(AsyncMessage::Notification(_))) => {
                    state.gateway.notifier.notify_waiters();
                }
                Some(Ok(AsyncMessage::Notice(notice))) => {
                    log::info!("Database notice: {notice}");
                }
                Some(Ok(_)) => unreachable!("AsyncMessage is non-exhaustive"),
                Some(Err(e)) => {
                    log::error!("Database connection error: {e}");
                    return Err(e.into());
                }
                None => break,
            }
        }

        Ok::<(), Error>(())
    }))
}
