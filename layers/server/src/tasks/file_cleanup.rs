use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::Error;

use super::*;

pub fn add_orphaned_file_cleanup_task(state: &ServerState, runner: &TaskRunner) {
    if state.config.upload.cleanup_interval.is_zero() {
        return;
    }

    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        state.config.upload.cleanup_interval,
        |state, _, _| async move {
            log::trace!("Cleaning up orphaned files");

            let task = async {
                // find orphaned files

                let db = state.db.read.get().await?;
                let orphaned = db
                    .query_cached_typed(
                        || {
                            use schema::*;
                            use thorn::*;

                            Query::select().from_table::<Files>().col(Files::Id).and_where(
                                Files::Id.not_in_query(
                                    Query::select().from_table::<AggUsedFiles>().col(AggUsedFiles::Id),
                                ),
                            )
                        },
                        &[],
                    )
                    .await?;

                // try to delete files

                let _fs_permit = state.fs_semaphore.acquire().await?;
                let fs = state.fs();

                let mut deleted = Vec::new();
                let deleting = async {
                    for row in orphaned {
                        let id = row.try_get(0)?;
                        match fs.delete(id).await {
                            Ok(_) => {}
                            Err(ref e) if e.kind() != std::io::ErrorKind::NotFound => {
                                // allow already-deleted files to be cleaned
                            }
                            Err(e) => return Err(e.into()),
                        }
                        deleted.push(id);
                    }

                    Ok::<(), Error>(())
                };

                let res = deleting.await;

                // if any were deleted successfully, DELETE from database
                if !deleted.is_empty() {
                    let db = state.db.write.get().await?;

                    db.execute_cached_typed(
                        || {
                            use schema::*;
                            use thorn::*;

                            Query::delete().from::<Files>().and_where(
                                Query::select()
                                    .expr(Var::of(SNOWFLAKE_ARRAY))
                                    .any()
                                    .equals(Files::Id),
                            )
                        },
                        &[&deleted],
                    )
                    .await?;

                    log::info!("Deleted {} orphaned files!", deleted.len());
                }

                res
            };

            if let Err(e) = task.await {
                log::error!("Error cleaning orphaned files: {e}");
            }
        },
    )))
}
