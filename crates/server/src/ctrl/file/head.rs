use std::io::ErrorKind;

use crate::{
    ctrl::Error,
    web::{auth::Authorization, routes::api::v1::file::post::Metadata},
    ServerState,
};

use schema::Snowflake;

pub struct UploadHead {
    pub size: i32,
    pub flags: i16,
    pub name: String,
    pub mime: Option<String>,
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
        let _file_lock = state.id_lock.lock(file_id).await;

        match state.fs.metadata(file_id).await {
            Ok(meta) => Ok(Some(meta)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    };

    let (mut head, metadata) = tokio::try_join!(fetch_record, fetch_metadata)?;

    if let Some(meta) = metadata {
        head.offset = meta.len() as i32;
    }

    Ok(head)
}
