use ftl::*;
use headers::HeaderValue;
use models::Snowflake;

use crate::{
    ctrl::{file::patch::FilePatchParams, Error},
    web::{auth::Authorization, routes::api::ApiError},
    ServerState,
};

pub async fn patch(
    mut route: Route<ServerState>,
    auth: Authorization,
    file_id: Snowflake,
) -> Result<Response, Error> {
    match route.raw_header("Content-Type") {
        Some(ct) if ct.as_bytes() == b"application/offset+octet-stream" => {}
        _ => return Ok(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response()),
    }

    let upload_offset: u32 = match route.parse_raw_header("Upload-Offset") {
        Some(Ok(Ok(upload_offset))) => upload_offset,
        _ => return Ok(StatusCode::BAD_REQUEST.into_response()),
    };

    let checksum = match route.raw_header("Upload-Checksum") {
        Some(checksum_header) => parse_checksum_header(checksum_header)?,
        _ => return Ok(StatusCode::BAD_REQUEST.into_response()),
    };

    let content_length = match route.header::<headers::ContentLength>() {
        Some(cl) => cl.0,
        _ => return Ok(StatusCode::BAD_REQUEST.into_response()),
    };

    if content_length > (route.state.config.max_upload_chunk_size as u64) {
        return Err(Error::RequestEntityTooLarge);
    }

    let params = FilePatchParams {
        crc32: checksum,
        upload_offset,
        content_length,
    };

    let body = route.take_body().unwrap();

    let patch = crate::ctrl::file::patch::patch_file(route.state, auth, file_id, params, body).await?;

    let mut res = Response::default();
    *res.status_mut() = StatusCode::NO_CONTENT;

    let headers = res.headers_mut();

    headers.extend(super::TUS_HEADERS.iter().map(|(k, v)| (k.clone(), v.clone())));

    headers.insert("Upload-Offset", super::header_from_int(patch.upload_offset));

    Ok(res)
}

fn parse_checksum_header(header: &HeaderValue) -> Result<u32, Error> {
    // Upload-Checksum: crc32 sawegsdgsdgsdg=
    let mut parts = header.to_str()?.split(' ').map(str::trim);

    // TODO: Maybe support alternatives eventually?
    if parts.next() != Some("crc32") {
        return Err(Error::ChecksumMismatch);
    }

    let checksum_encoded = match parts.next() {
        Some(s) => s,
        None => return Err(Error::ChecksumMismatch),
    };

    let mut out = [0u8; 4];
    if base64::decode_config_slice(checksum_encoded, base64::STANDARD, &mut out)? != 4 {
        return Err(Error::ChecksumMismatch);
    }

    Ok(u32::from_be_bytes(out))
}
