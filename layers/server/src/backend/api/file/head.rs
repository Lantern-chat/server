use std::io::ErrorKind;

use crate::{Authorization, Error, ServerState};

use schema::Snowflake;
use smol_str::SmolStr;

pub struct UploadHead {
    pub size: i32,
    pub flags: i16,
    pub name: SmolStr,
    pub mime: Option<SmolStr>,
    pub preview: Option<Vec<u8>>,
    pub offset: i32,
}

pub async fn head(state: ServerState, auth: Authorization, file_id: Snowflake) -> Result<UploadHead, Error> {
    let fetch_record = async {
        let db = state.db.read.get().await?;

        let row = db
            .query_opt_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    Query::select()
                        .from_table::<Files>()
                        .cols(&[
                            Files::Size,
                            Files::Flags,
                            Files::Name,
                            Files::Mime,
                            Files::Preview,
                        ])
                        .and_where(Files::Id.equals(Var::of(Files::Id)))
                        .and_where(Files::UserId.equals(Var::of(Files::UserId)))
                        .limit_n(1)
                },
                &[&file_id, &auth.user_id],
            )
            .await?;

        match row {
            None => Err(Error::NotFound),
            Some(row) => Ok(UploadHead {
                size: row.try_get(0)?,
                flags: row.try_get(1)?,
                name: row.try_get(2)?,
                mime: row.try_get(3)?,
                preview: row.try_get(4)?,
                offset: 0,
            }),
        }
    };

    let fetch_metadata = async {
        // TODO: Determine if _file_lock is necessary, as HEAD can be called on a completed file

        // acquire these at the same time
        let (_file_lock, _fs_permit) = tokio::try_join! {
            async { Ok(state.id_lock.lock(file_id).await) },
            async { state.fs_semaphore.acquire().await },
        }?;

        match state.fs().metadata(file_id).await {
            Ok(meta) => Ok(Some(meta)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    };

    let (mut head, metadata) = tokio::try_join!(fetch_record, fetch_metadata)?;

    if let Some(meta) = metadata {
        head.offset = meta.len() as i32;
    } else {
        log::trace!("File HEAD on file that doesn't exist yet");
    }

    Ok(head)
}
