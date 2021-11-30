use std::io::Cursor;

use models::Snowflake;
use schema::{flags::FileFlags, SnowflakeExt};
use smol_str::SmolStr;

use crate::{
    ctrl::Error,
    filesystem::store::{CipherOptions, OpenMode},
    ServerState,
};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn process_avatar(state: ServerState, user_id: Snowflake, file_id: Snowflake) -> Result<(), Error> {
    let read_db = state.db.read.get().await?;

    let row = read_db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Files>()
                    .cols(&[
                        Files::Size,
                        Files::Nonce,
                        Files::Flags,
                        Files::Mime,
                        Files::Preview,
                    ])
                    .and_where(Files::UserId.equals(Var::of(Files::UserId)))
                    .and_where(Files::Id.equals(Var::of(Files::Id)))
                    .limit_n(1)
            },
            &[&user_id, &file_id],
        )
        .await?;

    let row = match row {
        Some(row) => row,
        None => return Err(Error::NotFound),
    };

    let size: i32 = row.try_get(0)?;

    if size > state.config.max_avatar_size {
        return Err(Error::RequestEntityTooLarge);
    }

    let nonce: i64 = row.try_get(1)?;
    //let flags = FileFlags::from_bits_truncate(row.try_get(2)?);
    let mime: Option<&str> = row.try_get(3)?;

    // TODO: Use mime type somehow?
    let _mime = match mime {
        Some(mime) => mime,
        None => return Err(Error::MissingMime),
    };

    let preview: Option<Vec<u8>> = row.try_get(4)?;

    let cipher_options = CipherOptions {
        key: state.config.file_key,
        nonce: unsafe { std::mem::transmute([nonce, nonce]) },
    };

    let _fs_permit = state.fs_semaphore.acquire().await?;

    let mut file = state
        .fs
        .open_crypt(file_id, OpenMode::Read, &cipher_options)
        .await?;

    let mut buffer = Vec::with_capacity(size as usize);
    file.read_to_end(&mut buffer).await?;

    drop((file, _fs_permit));

    let _processing_permit = state.processing_semaphore.acquire().await?;

    let encode_state = state.clone();
    let encode_task = tokio::task::spawn_blocking(move || -> Result<_, Error> {
        use processing::avatar::{EncodedImage, ProcessConfig, ProcessingError};
        use processing::read_image::ImageReadError;

        processing::avatar::process_avatar(
            buffer,
            preview,
            ProcessConfig {
                max_width: encode_state.config.max_avatar_width,
                max_pixels: encode_state.config.max_avatar_pixels,
            },
        )
        .map_err(|err| match err {
            ProcessingError::InvalidImageFormat
            | ProcessingError::ImageReadError(ImageReadError::InvalidImageFormat) => {
                Error::InvalidImageFormat
            }
            ProcessingError::TooLarge | ProcessingError::ImageReadError(ImageReadError::ImageTooLarge) => {
                Error::RequestEntityTooLarge
            }
            ProcessingError::Other(e) => Error::InternalError(e),
            ProcessingError::ImageReadError(_) => Error::BadRequest,
        })
    });

    let processing::avatar::ProcessedImage {
        reused,
        image: encoded_image,
    } = encode_task.await??;

    let new_file = !reused;

    drop(_processing_permit);

    let mut avatar_file_id = file_id;

    if new_file {
        let (new_file_id, nonce) = crate::ctrl::file::post::do_post_file(
            state.clone(),
            user_id,
            encoded_image.buffer.len() as i32,
            format!("{}_avatar.png", user_id).into(),
            Some(SmolStr::new_inline("image/png")),
            None,
            Some(encoded_image.width as i32),
            Some(encoded_image.height as i32),
        )
        .await?;

        avatar_file_id = new_file_id;

        let _fs_permit = state.fs_semaphore.acquire().await?;

        let cipher_options = CipherOptions {
            key: state.config.file_key,
            nonce: unsafe { std::mem::transmute([nonce, nonce]) },
        };

        let mut file = state
            .fs
            .open_crypt(avatar_file_id, OpenMode::Write, &cipher_options)
            .await?;

        file.write_all(&encoded_image.buffer).await?;
        file.flush().await?;
    }

    let db = state.db.write.get().await?;

    if new_file {
        db.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::update()
                    .table::<Files>()
                    .set(Files::Flags, Var::of(Files::Flags))
                    .set(Files::Preview, Var::of(Files::Preview))
                    .and_where(Files::Id.equals(Var::of(Files::Id)))
            },
            &[
                &FileFlags::COMPLETE.bits(),
                &encoded_image.preview,
                &avatar_file_id,
            ],
        )
        .await?;
    }

    db.execute_cached_typed(
        || {
            use schema::*;
            use thorn::*;

            Query::call(Call::custom("lantern.upsert_user_avatar").args((
                Var::of(Users::Id),
                Var::of(Party::Id),
                Var::of(Files::Id),
            )))
        },
        &[&user_id, &Option::<i64>::None, &avatar_file_id],
    )
    .await?;

    Ok(())
}
