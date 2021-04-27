use futures::StreamExt;
use std::io::ErrorKind;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use ftl::*;

use db::{schema::file::File, Snowflake};

pub async fn patch(mut route: Route<crate::ServerState>, mut file: File) -> impl Reply {
    match route.raw_header("Content-Type") {
        Some(ct) if ct.as_bytes() == b"application/offset+octet-stream" => {}
        _ => return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response(),
    }

    let upload_offset: u32 = match route.parse_raw_header("Upload-Offset") {
        Some(Ok(Ok(upload_offset))) => upload_offset,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    if upload_offset != file.offset {
        return StatusCode::CONFLICT.into_response();
    }

    let content_length = match route.header::<headers::ContentLength>() {
        Some(cl)
            if cl.0 > route.state.config.max_upload_size as u64
                || (cl.0 as u32 + upload_offset) <= file.size =>
        {
            cl.0
        }
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    let mut fd = match route
        .state
        .fs
        .open(file.id, file.offset as u64, false)
        .await
    {
        Ok(f) => f,
        Err(e) => {
            let res = if e.kind() == ErrorKind::NotFound {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            return res.into_response();
        }
    };

    let mut stream = match route.stream() {
        Ok(s) => s,
        Err(e) => {
            log::error!("Patch file error: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // TODO: Parse Upload-Checksum
    //let mut crc = crc32fast::Hasher::new();
    let mut written = 0;

    while let Some(chunk) = stream.next().await {
        let mut chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                log::error!("Error receiving file stream: {}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

        //crc.update(&chunk);

        // TODO: Encrypt content

        match fd.write_buf(&mut chunk).await {
            Ok(n) => written += n,
            Err(e) => {
                log::error!("Error forwarding file stream: {}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    }

    // TODO: Compare checksums

    if written != content_length as usize {
        log::warn!(
            "File Upload terminated earlier than expected, {} out of {} bytes written",
            written,
            content_length
        );
    }

    file.offset += written as u32;

    if let Err(e) = file.update_offset(&route.state.db).await {
        log::error!("Error updating file offset: {}", e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // just copy formatting from the HEAD response
    let mut res = super::head::head(route, file).await.into_response();
    *res.status_mut() = StatusCode::NO_CONTENT;

    res
}
