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
    let flags = FileFlags::PARTIAL.bits();

    #[rustfmt::skip]
    state.db.write.get().await?.execute2(thorn::sql! {
        use schema::*;

        INSERT INTO Files (Id, UserId, Nonce, Size, Width, Height, Flags, Name, Mime, Preview)
        VALUES (
            #{&file_id      => Files::Id},
            #{&user_id      => Files::UserId},
            #{&nonce        => Files::Nonce},
            #{&upload_size  => Files::Size},
            #{&width        => Files::Width},
            #{&height       => Files::Height},
            #{&flags        => Files::Flags},
            #{&filename     => Files::Name},
            #{&mime         => Files::Mime},
            #{&preview      => Files::Preview}
        )
    }?).await?;

    Ok((file_id, nonce))
}
