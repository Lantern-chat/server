use std::{error::Error as _, io::SeekFrom};

use bytes::Bytes;
use filesystem::store::{CipherOptions, FileExt, OpenMode};

use crate::{Authorization, Error, ServerState};

use futures::{FutureExt, Stream, StreamExt};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

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

    #[rustfmt::skip]
    let row = state.db.read.get().await?.query_opt2(thorn::sql! {
        use schema::*;

        SELECT
            Files.Size  AS @Size,
            Files.Flags AS @Flags,
            Files.Nonce AS @Nonce,

            Files.Mime IS NOT NULL AS @NoMime
        FROM Files
        WHERE
            Files.Id     = #{&file_id => Files::Id}
        AND Files.UserId = #{&auth.user_id => Files::UserId}
    }?).await?;

    let Some(row) = row else { return Err(Error::NotFound) };

    let size = row.size::<i32>()? as u64;
    let mut flags: FileFlags = FileFlags::from_bits_truncate(row.flags()?);
    let nonce: i64 = row.nonce()?;
    let no_mime: bool = row.no_mime()?;

    // a completed file cannot be modified
    if flags.contains(FileFlags::COMPLETE) {
        return Err(Error::Conflict);
    }

    // acquire these at the same time
    let (_file_lock, _fs_permit) = tokio::join! {
        state.id_lock.lock(file_id),
        state.fs_semaphore.acquire(),
    };

    let _fs_permit = _fs_permit?;

    let mut file = state
        .fs()
        .open_crypt(
            file_id,
            OpenMode::Write,
            &CipherOptions::new_from_i64_nonce(state.config().keys.file_key, nonce),
        )
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

    // small chunk of buffered content for mime deduction
    let mut prefix = Vec::new();
    let read_prefix = no_mime && append_pos == 0;

    // only allocate if necessary, and no more than needed
    if read_prefix {
        prefix = vec![0u8; 260];
    }

    let mut res = loop {
        let chunk = body.next().await;

        match chunk {
            None => break None,
            Some(Err(e)) => {
                let is_fatal = e.is_parse() || e.is_parse_status() || e.is_parse_too_large() || e.is_user();

                break Some(if is_fatal {
                    Error::InternalError(e.message().to_string())
                } else {
                    Error::UploadError
                });
            }
            Some(Ok(bytes)) => {
                let num_bytes = bytes.len() as u64;
                let new_bytes_written = bytes_written + num_bytes;

                // check if request is too large before writing
                if new_bytes_written > params.content_length {
                    break Some(Error::RequestEntityTooLarge);
                }

                // copy parts of the first chunk into the given prefix buffer
                // if `bytes_written` is past the first_chunk length, it's already filled, so skip
                if read_prefix && bytes_written < prefix.len() as u64 {
                    // start of the range that needs to be filled
                    let prefix_start = bytes_written as usize;
                    // end at the max length or where these bytes can fill ends
                    let prefix_end = prefix.len().min(prefix_start + bytes.len());

                    // if we don't use the entirity of the given bytes, slice it
                    let len = prefix_end - prefix_start;

                    prefix[prefix_start..prefix_end].copy_from_slice(&bytes[..len]);
                }

                // update crc before bytes are moved out
                crc32.update(&bytes);

                if let Err(e) = file.write_buf(&bytes).await {
                    break Some(e.into());
                }

                bytes_written = new_bytes_written;
            }
        }
    };

    if let Err(e) = file.flush().await {
        match res {
            Some(Error::IOError(_)) => {
                log::error!("Error flushing file: {e}, probably due to previous IO error")
            }
            Some(_) => log::error!("Error flushing file after non-io error: {e}"),
            None => res = Some(e.into()),
        }
    }

    // check checksum
    let final_crc32 = crc32.finalize();
    if params.crc32 != final_crc32 {
        log::debug!("{:X} != {:X}", params.crc32, final_crc32);

        res = res.or(Some(Error::ChecksumMismatch));
    }

    if let Some(err) = res {
        // only rewind if there was anything written
        if bytes_written > 0 {
            file.set_len(append_pos).boxed().await?;
        }

        return Err(err);
    }

    drop((file, _fs_permit));

    let mut file_patch = FilePatch {
        complete: false,
        upload_offset: end_pos,
    };

    if read_prefix {
        // need to find the prefix end if it wasn't filled fully (small files)
        let prefix_end = prefix.len().min(bytes_written as usize);
        let first_chunk = &prefix[..prefix_end];

        // try to deduce mime type from initial bytes
        if let Some((mstr, _)) = mime_db::from_prefix(first_chunk) {
            #[rustfmt::skip]
            state.db.write.get().await?.execute2(thorn::sql! {
                use schema::*;
                UPDATE Files SET (Mime) = (#{&mstr => Files::Mime})
                WHERE Files.Id = #{&file_id => Files::Id}
            }?).await?;
        }
    }

    // the file has finished uploading, so mark it as complete
    if end_pos == size {
        flags.remove(FileFlags::PARTIAL);
        flags.insert(FileFlags::COMPLETE);

        let bits = flags.bits();

        #[rustfmt::skip]
        state.db.write.get().await?.execute2(thorn::sql! {
            use schema::*;
            UPDATE Files SET (Flags) = (#{&bits => Files::Flags})
            WHERE Files.Id = #{&file_id => Files::Id}
        }?).await?;

        file_patch.complete = true;
    }

    drop(_file_lock);

    crate::metrics::API_METRICS.load().upload.add(params.content_length);

    Ok(file_patch)
}
