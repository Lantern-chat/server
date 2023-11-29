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
        #[rustfmt::skip]
        let row = state.db.read.get().await?.query_opt2(schema::sql! {
            SELECT
                Files.Size    AS @Size,
                Files.Flags   AS @Flags,
                Files.Name    AS @Name,
                Files.Mime    AS @Mime,
                Files.Preview AS @Preview
            FROM  Files
            WHERE Files.Id     = #{&file_id as Files::Id}
              AND Files.UserId = #{auth.user_id_ref() as Files::UserId}
        }).await?;

        match row {
            None => Err(Error::NotFound),
            Some(row) => Ok(UploadHead {
                size: row.size()?,
                flags: row.flags()?,
                name: row.name()?,
                mime: row.mime()?,
                preview: row.preview()?,
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
