use std::str::FromStr;

use crate::{Error, ServerState};

use schema::{flags::FileFlags, Snowflake, SnowflakeExt};

use rand::Rng;
use smol_str::SmolStr;

#[derive(Debug, Deserialize)]
pub struct FilePostBody {
    filename: SmolStr,

    size: i32,

    #[serde(default)]
    mime: Option<SmolStr>,

    #[serde(default)]
    width: Option<i32>,

    #[serde(default)]
    height: Option<i32>,

    #[serde(default)]
    preview: Option<String>,
}

pub async fn post_file(state: &ServerState, user_id: Snowflake, body: FilePostBody) -> Result<Snowflake, Error> {
    let mime = match body.mime {
        None => None,
        Some(mime) => {
            // try parsing the mime given type
            let _ = mime::Mime::from_str(&mime)?;

            Some(mime)
        }
    };

    let preview = match body.preview {
        None => None,
        Some(preview) => Some({
            use blurhash::decode;
            use z85::FromZ85;

            let preview = preview.from_z85()?;

            if !decode::is_valid(&preview)? {
                return Err(Error::InvalidPreview);
            }

            preview
        }),
    };

    match do_post_file(
        state,
        user_id,
        body.size,
        body.filename,
        mime,
        preview,
        body.width,
        body.height,
    )
    .await
    {
        Ok((file_id, _nonce)) => Ok(file_id),
        Err(e) => Err(e),
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn do_post_file(
    state: &ServerState,
    user_id: Snowflake,
    upload_size: i32,
    filename: SmolStr,
    mime: Option<SmolStr>,
    preview: Option<Vec<u8>>,
    width: Option<i32>,
    height: Option<i32>,
) -> Result<(Snowflake, i64), Error> {
    let file_id = Snowflake::now();
    let nonce: i64 = util::rng::crypto_thread_rng().gen();

    let db = state.db.write.get().await?;

    mod post_file_query {
        pub use schema::*;
        pub use thorn::*;

        use smol_str::SmolStr;

        thorn::params! {
            pub struct Params {
                pub file_id: Snowflake = Files::Id,
                pub user_id: Snowflake = Files::UserId,
                pub nonce: i64 = Files::Nonce,
                pub size: i32 = Files::Size,
                pub width: Option<i32> = Files::Width,
                pub height: Option<i32> = Files::Height,
                pub flags: i16 = Files::Flags,
                pub filename: SmolStr = Files::Name,
                pub mime: Option<SmolStr> = Files::Mime,
                pub preview: Option<Vec<u8>> = Files::Preview,
            }
        }
    }

    use post_file_query::{Parameters, Params};

    // TODO: Add flags
    db.execute_cached_typed(
        || {
            use post_file_query::*;

            Query::insert()
                .into::<Files>()
                .cols(&[
                    Files::Id,
                    Files::UserId,
                    Files::Nonce,
                    Files::Size,
                    Files::Width,
                    Files::Height,
                    Files::Flags,
                    Files::Name,
                    Files::Mime,
                    Files::Preview,
                ])
                .values([
                    Params::file_id(),
                    Params::user_id(),
                    Params::nonce(),
                    Params::size(),
                    Params::width(),
                    Params::height(),
                    Params::flags(),
                    Params::filename(),
                    Params::mime(),
                    Params::preview(),
                ])
        },
        &Params {
            file_id,
            user_id,
            nonce,
            size: upload_size,
            width,
            height,
            flags: FileFlags::PARTIAL.bits(),
            filename,
            mime,
            preview,
        }
        .as_params(),
    )
    .await?;

    Ok((file_id, nonce))
}
