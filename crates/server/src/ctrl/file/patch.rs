use std::io::SeekFrom;

use crate::{
    ctrl::Error,
    filesystem::store::{CipherOptions, FileExt, OpenMode, RWSeekStream},
    web::{auth::Authorization, routes::api::v1::file::post::Metadata},
    ServerState,
};

use futures::{Stream, StreamExt};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use aes::{cipher::Nonce, Aes256Ctr};
use bytes::Bytes;
use schema::{flags::FileFlags, Snowflake, SnowflakeExt};

pub struct FilePatch {
    pub complete: bool,
    pub upload_offset: u64,
}

pub struct FilePatchParams {
    pub crc32: u32,
    pub upload_offset: u32,
    pub content_length: u64,
}

pub async fn patch_file(
    state: ServerState,
    auth: Authorization,
    file_id: Snowflake,
    params: FilePatchParams,
    mut body: hyper::Body,
) -> Result<FilePatch, Error> {
    let db = state.db.read.get().await?;

    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Files>()
                    .cols(&[Files::Size, Files::Flags, Files::Nonce])
                    .and_where(Files::Id.equals(Var::of(Files::Id)))
                    .and_where(Files::UserId.equals(Var::of(Files::UserId)))
            },
            &[&file_id, &auth.user_id],
        )
        .await?;

    let row = match row {
        Some(row) => row,
        None => return Err(Error::NotFound),
    };

    let size = row.try_get::<_, i32>(0)? as u64;
    let mut flags: FileFlags = FileFlags::from_bits_truncate(row.try_get(1)?);
    let nonce: i64 = row.try_get(2)?;

    drop(db); // free connection

    // a completed file cannot be modified
    if flags.contains(FileFlags::COMPLETE) {
        return Err(Error::Conflict);
    }

    let cipher_options = CipherOptions {
        key: state.config.file_key,
        nonce: unsafe { std::mem::transmute([nonce, nonce]) },
    };

    let _file_lock = state.id_lock.lock(file_id).await;

    let mut file = state
        .fs
        .open_crypt(file_id, OpenMode::Write, &cipher_options)
        .await?;

    let append_pos = file.seek(SeekFrom::End(0)).await?;

    if params.upload_offset as u64 != append_pos {
        return Err(Error::Conflict);
    }

    let end_pos = append_pos + params.content_length;

    // Don't allow excess writing
    if end_pos > size {
        return Err(Error::RequestEntityTooLarge);
    }

    let mut crc32 = crc32fast::Hasher::new();
    let mut bytes_written = 0;

    let mut res = loop {
        match body.next().await {
            None => break None,
            Some(Err(e)) => {
                let is_fatal = e.is_parse() || e.is_parse_status() || e.is_parse_too_large() || e.is_user();

                break Some(if is_fatal {
                    Error::InternalError(e.to_string())
                } else {
                    Error::UploadError
                });
            }
            Some(Ok(mut bytes)) => {
                let num_bytes = bytes.len() as u64;
                let new_bytes_written = bytes_written + num_bytes;

                // check if request is too large before writing
                if new_bytes_written > params.content_length {
                    break Some(Error::RequestEntityTooLarge);
                }

                // update crc before bytes are moved out
                crc32.update(&bytes);

                if let Err(e) = file.write_all_buf(&mut bytes).await {
                    break Some(e.into());
                }

                bytes_written = new_bytes_written;
            }
        }
    };

    if let Err(e) = file.flush().await {
        match res {
            Some(Error::IOError(_)) => {
                log::error!("Error flushing file: {}, probably due to previous IO error", e)
            }
            Some(_) => log::error!("Error flushing file after non-io error: {}", e),
            None => res = Some(e.into()),
        }
    }

    // check checksum
    if params.crc32 != crc32.finalize() {
        res = res.or(Some(Error::ChecksumMismatch));
    }

    if let Some(err) = res {
        // only rewind if there was anything written
        if bytes_written > 0 {
            file.set_len(append_pos).await?;
        }

        return Err(err);
    }

    let mut file_patch = FilePatch {
        complete: false,
        upload_offset: end_pos,
    };

    if end_pos == size {
        let db = state.db.write.get().await?;

        flags.remove(FileFlags::PARTIAL);
        flags.insert(FileFlags::COMPLETE);

        db.execute_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::update()
                    .table::<Files>()
                    .set(Files::Flags, Var::of(Files::Flags))
                    .and_where(Files::Id.equals(Var::of(Files::Id)))
            },
            &[&flags.bits(), &file_id],
        )
        .await?;

        file_patch.complete = true;
    }

    Ok(file_patch)
}
