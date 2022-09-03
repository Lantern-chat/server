use thorn::conflict::ConflictAction;

use super::*;

pub fn add_emoji_insert_task(state: ServerState, runner: &TaskRunner) {
    runner.add(AsyncFnTask::new(|_| async move {
        log::trace!("Refreshing emoji list");

        let res = async {
            let db = state.db.write.get().await?;

            let emojis: Vec<_> = emoji::iter().collect();

            db.execute_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    Query::insert()
                        .into::<Emojis>()
                        .cols(&[Emojis::Emoji])
                        .value(Builtin::unnest(Var::of(Type::TEXT_ARRAY)))
                        .on_conflict([Emojis::Emoji], ConflictAction::DoNothing)
                },
                &[&emojis],
            )
            .await?;

            Ok::<(), crate::Error>(())
        };

        if let Err(e) = res.await {
            log::error!("Error inserting emojis: {}", e);
        }
    }))
}
