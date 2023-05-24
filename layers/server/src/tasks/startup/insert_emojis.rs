use super::*;

pub fn add_emoji_insert_task(state: ServerState, runner: &TaskRunner) {
    runner.add(AsyncFnTask::new(|_| async move {
        log::trace!("Refreshing emoji list");

        let res = async {
            let emojis: Vec<_> = emoji::iter().collect();

            let db = state.db.write.get().await?;

            db.execute2(schema::sql! {
                INSERT INTO Emojis (Emoji) VALUES ( UNNEST(#{&emojis as Type::TEXT_ARRAY}) )
                ON CONFLICT DO NOTHING
            })
            .await?;

            Ok::<(), crate::Error>(())
        };

        if let Err(e) = res.await {
            log::error!("Error inserting emojis: {}", e);
        }
    }))
}
