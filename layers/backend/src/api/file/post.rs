use std::str::FromStr;

use crate::{Error, State};

use db::pool::Object;
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

pub async fn post_file(state: State, user_id: Snowflake, body: FilePostBody) -> Result<Snowflake, Error> {
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
            use blurhash::{base85::*, decode};

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

pub async fn do_post_file(
    state: State,
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
                    Files::Width,
                    Files::Height,
                    Files::Flags,
                    Files::Name,
                    Files::Mime,
                    Files::Preview,
                ])
                .values([
                    Var::of(Files::Id),
                    Var::of(Files::UserId),
                    Var::of(Files::Nonce),
                    Var::of(Files::Size),
                    Var::of(Files::Width),
                    Var::of(Files::Height),
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
            &width,
            &height,
            &FileFlags::PARTIAL.bits(),
            &filename,
            &mime,
            &preview,
        ],
    )
    .await?;

    Ok((file_id, nonce))
}
