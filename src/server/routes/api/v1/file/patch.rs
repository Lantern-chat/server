use futures::StreamExt;
use http::StatusCode;
use std::io::ErrorKind;
use tokio::io::AsyncWriteExt;

use crate::{
    db::{schema::file::File, Snowflake},
    server::ftl::*,
};

pub async fn patch(mut route: Route, mut file: File) -> impl Reply {
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
        .open(file.id, upload_offset as u64, false)
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
    let mut crc = crc32fast::Hasher::new();
    let mut written = 0;

    while let Some(chunk) = stream.next().await {
        let mut chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                log::error!("Error receiving file stream: {}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

        crc.update(&chunk);

        match fd.write_buf(&mut chunk).await {
            Ok(n) => written += n,
            Err(e) => {
                log::error!("Error forwarding file stream: {}", e);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
    }

    file.offset += written as u32;

    if written != content_length as usize {
        // TODO: Server received partial request
    }

    ().into_response()
}
