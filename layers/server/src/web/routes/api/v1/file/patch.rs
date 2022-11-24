use ftl::*;
use headers::{ContentLength, HeaderValue};
use sdk::models::Snowflake;

use super::ApiResponse;
use crate::backend::api::file::patch::FilePatchParams;
use crate::{Authorization, Error, ServerState};

lazy_static::lazy_static! {
    static ref APPLICATION_OFFSET_OCTET_STREAM: headers::ContentType =
        headers::ContentType::from("application/offset+octet-stream".parse::<mime::Mime>().unwrap());
}

pub async fn patch(mut route: Route<ServerState>, auth: Authorization, file_id: Snowflake) -> ApiResponse {
    match route.header::<headers::ContentType>() {
        None => return Err(Error::MissingContentTypeHeader),
        Some(ct) if ct == *APPLICATION_OFFSET_OCTET_STREAM => {}
        Some(ct) => return Err(Error::UnsupportedMediaType(ct)),
    }

    let Some(Ok(Ok(upload_offset))) = route.parse_raw_header("Upload-Offset") else { return Err(Error::BadRequest) };
    let Some(ContentLength(content_length)) = route.header::<headers::ContentLength>() else { return Err(Error::BadRequest) };

    let checksum = match route.raw_header("Upload-Checksum") {
        Some(checksum_header) => parse_checksum_header(checksum_header)?,
        _ => return Err(Error::BadRequest),
    };

    if content_length > (route.state.config.upload.max_upload_chunk_size as u64) {
        return Err(Error::RequestEntityTooLarge);
    }

    let params = FilePatchParams {
        crc32: checksum,
        upload_offset,
        content_length,
    };

    let body = route.take_body().unwrap();

    let patch =
        crate::backend::api::file::patch::patch_file(route.state, auth, file_id, params, body).await?;

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

    let Some(checksum_encoded) = parts.next() else { return Err(Error::ChecksumMismatch) };

    let mut out = [0u8; 4];
    if 4 != base64::decode_config_slice(checksum_encoded, base64::STANDARD, &mut out)? {
        return Err(Error::ChecksumMismatch);
    }

    Ok(u32::from_be_bytes(out))
}
