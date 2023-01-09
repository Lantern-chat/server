use std::{io::SeekFrom, str::FromStr, time::Instant};

use bytes::{Bytes, BytesMut};
use ftl::{
    fs::{bytes_range, Cond, Conditionals},
    *,
};

use filesystem::store::{CipherOptions, FileExt, OpenMode};
use futures::FutureExt;
use headers::{
    AcceptRanges, ContentLength, ContentRange, ContentType, HeaderMap, HeaderMapExt, HeaderValue, IfModifiedSince,
    LastModified, Range,
};
use hyper::Body;
use smol_str::SmolStr;
use thorn::pg::ToSql;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use schema::flags::FileFlags;
use sdk::models::Snowflake;

use crate::{Error, ServerState};

pub async fn send_file(
    state: ServerState,
    meta: super::ParsedFile<'_>,
    is_head: bool,
    download: bool,
    range: Option<Range>,
    last_modified: LastModified,
    start_time: Option<Instant>,
) -> Result<Response, Error> {
    let _fs_permit = state.fs_semaphore.acquire().await?;

    let mut file = state
        .fs()
        .open_crypt(
            meta.id,
            OpenMode::Read,
            &CipherOptions::new_from_i64_nonce(state.config().keys.file_key, meta.nonce),
        )
        .await?;

    let mut len = meta.size as u64;

    // in debug mode, double-check length
    if cfg!(debug_assertions) {
        let real_len = file.get_len().await?;

        assert_eq!(len, real_len);
    }

    let mut res = if is_head {
        Response::default()
    } else {
        // parse byte range using ftl method
        let (start, end) = match bytes_range(range, len) {
            Err(_) => {
                return Ok(StatusCode::RANGE_NOT_SATISFIABLE
                    .with_header(ContentRange::unsatisfied_bytes(len))
                    .into_response())
            }
            Ok(range) => range,
        };

        // determine content length from range (if applicable)
        let sub_len = end - start;

        // setup body, sender and response
        let (mut sender, body) = Body::channel();
        let mut res = Response::new(body);

        // if the selected range is not the entire length, set applicable headers
        if len != sub_len {
            *res.status_mut() = StatusCode::PARTIAL_CONTENT;

            res.headers_mut()
                .typed_insert(ContentRange::bytes(start..end, len).expect("valid ContentRange"));

            len = sub_len;
        }

        tokio::spawn(async move {
            if start != 0 {
                if let Err(e) = file.seek(SeekFrom::Start(start)).await {
                    log::error!("Error seeking file: {e}");
                    return sender.abort();
                }
            }

            let mut buf = BytesMut::new();
            let mut len = sub_len;

            let buf_size = 1024 * 512; // 512Kb

            while len != 0 {
                if buf.capacity() - buf.len() < buf_size {
                    buf.reserve(buf_size);
                }

                let n = match file.read_buf(&mut buf).await {
                    Ok(n) => n as u64,
                    Err(err) => {
                        log::error!("File read error: {err}");
                        return sender.abort();
                    }
                };

                if n == 0 {
                    log::warn!("File read found EOF before expected length: {len}");
                    break;
                }

                let mut chunk = buf.split().freeze();

                if n > len {
                    chunk = chunk.split_to(len as usize);
                    len = 0;
                } else {
                    len -= n;
                }

                if let Err(e) = sender.send_data(chunk).await {
                    log::trace!("Error sending file chunk: {e}");
                    return sender.abort();
                }
            }

            if let Some(start_time) = start_time {
                let elapsed = start_time.elapsed().as_secs_f64() * 1000.0;

                log::debug!("File transfer finished in {:.4}ms", elapsed);

                let mut trailers = HeaderMap::new();
                if let Ok(value) = HeaderValue::from_str(&format!("end;dur={:.4}", elapsed)) {
                    trailers.insert("Server-Timing", value);

                    if let Err(e) = sender.send_trailers(trailers).await {
                        log::trace!("Error sending trailers: {e}");
                    }
                } else {
                    log::trace!("Unable to create trailer value");
                }
            }

            drop(sender);
        });

        res
    };

    let headers = res.headers_mut();

    headers.insert("Cache-Control", HeaderValue::from_static("public, max-age=2678400"));
    headers.typed_insert(ContentLength(len));
    headers.typed_insert(AcceptRanges::bytes());
    headers.typed_insert(last_modified);

    // ensure filename in HTTP header is urlencoded for Unicode and such.
    let name = urlencoding::encode(meta.name);
    let cd = if download {
        format!("attachment; filename=\"{name}\"")
    } else {
        format!("inline; filename=\"{name}\"")
    };

    headers.insert("Content-Disposition", HeaderValue::from_str(&cd)?);

    if let Some(mime) = meta.mime {
        headers.insert("Content-Type", HeaderValue::from_str(mime)?);
    } else {
        headers.typed_insert(ContentType::octet_stream());
    }

    Ok(res)
}
