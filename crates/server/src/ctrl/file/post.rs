use crate::{
    ctrl::Error,
    web::{auth::Authorization, routes::api::v1::file::post::Metadata},
    ServerState,
};

use schema::{Snowflake, SnowflakeExt};

use rand::Rng;

pub async fn post_file(
    state: ServerState,
    auth: Authorization,
    upload_size: i32,
    metadata: Metadata<'_>,
) -> Result<Snowflake, Error> {
    let filename = String::from_utf8(base64::decode(metadata.filename)?)?;

    let mime = match metadata.mime {
        None => None,
        Some(mime) => Some(String::from_utf8(base64::decode(mime)?)?),
    };

    let preview = match metadata.preview {
        None => None,
        Some(preview) => Some({
            let preview = base64::decode(preview)?;

            if !blurhash::decode::is_valid(&preview)? {
                return Err(Error::InvalidPreview);
            }

            preview
        }),
    };

    let file_id = Snowflake::now();
    let nonce: i64 = util::rng::crypto_thread_rng().gen();

    let db = state.db.write.get().await?;

    // TODO: Add flags
    db.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;

            Query::insert()
                .into::<Files>()
                .cols(&[
                    Files::Id,
                    Files::UserId,
                    Files::Nonce,
                    Files::Size,
                    Files::Flags,
                    Files::Name,
                    Files::Mime,
                    Files::Preview,
                ])
                .values(vec![
                    Var::of(Files::Id),
                    Var::of(Files::UserId),
                    Var::of(Files::Nonce),
                    Var::of(Files::Size),
                    Var::of(Files::Flags),
                    Var::of(Files::Name),
                    Var::of(Files::Mime),
                    Var::of(Files::Preview),
                ])
        },
        &[
            &file_id,
            &auth.user_id,
            &nonce,
            &upload_size,
            &0i16,
            &filename,
            &mime,
            &preview,
        ],
    )
    .await?;

    Ok(file_id)
}
