use std::str::FromStr;

use crate::{ctrl::Error, web::routes::api::v1::file::post::Metadata, ServerState};

use db::pool::Object;
use schema::{flags::FileFlags, Snowflake, SnowflakeExt};

use rand::Rng;
use smol_str::SmolStr;

pub async fn post_file(
    state: ServerState,
    user_id: Snowflake,
    upload_size: i32,
    metadata: Metadata<'_>,
) -> Result<Snowflake, Error> {
    let filename = String::from_utf8(base64::decode(metadata.filename)?)?;

    let mime = match metadata.mime {
        None => None,
        Some(mime) => {
            let mime = String::from_utf8(base64::decode(mime)?)?;

            // try parsing the mime given type
            let _ = mime::Mime::from_str(&mime)?;

            Some(mime)
        }
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

    match do_post_file(state, user_id, upload_size, filename, mime, preview).await {
        Ok((file_id, _nonce)) => Ok(file_id),
        Err(e) => Err(e),
    }
}

pub async fn do_post_file(
    state: ServerState,
    user_id: Snowflake,
    upload_size: i32,
    filename: String,
    mime: Option<String>,
    preview: Option<Vec<u8>>,
) -> Result<(Snowflake, i64), Error> {
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
            &user_id,
            &nonce,
            &upload_size,
            &FileFlags::PARTIAL.bits(),
            &filename,
            &mime,
            &preview,
        ],
    )
    .await?;

    Ok((file_id, nonce))
}
