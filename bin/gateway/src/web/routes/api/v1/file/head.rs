use headers::{CacheControl, HeaderMapExt, HeaderName, HeaderValue};

use super::*;
use crate::{backend::api::file::head::UploadHead, util::TupleClone};

pub async fn head(route: Route<ServerState>, auth: Authorization, file_id: FileId) -> ApiResult {
    let head = crate::backend::api::file::head::head(route.state, auth, file_id).await?;

    let mut res = Response::default();

    let headers = res.headers_mut();

    headers.extend(super::tus_headers());

    headers.insert(HeaderName::from_static("upload-metadata"), encode_metadata(&head));

    headers.insert(
        HeaderName::from_static("upload-length"),
        super::header_from_int(head.size),
    );

    if head.size != head.offset {
        headers.insert(
            HeaderName::from_static("upload-offset"),
            super::header_from_int(head.offset),
        );
    }

    headers.typed_insert(CacheControl::new().with_no_store());

    Ok(res.into())
}

use base64::engine::{general_purpose::STANDARD, Engine};

fn encode_metadata(head: &UploadHead) -> HeaderValue {
    let mut value = "filename ".to_owned();
    STANDARD.encode_string(head.name.as_bytes(), &mut value);

    if let Some(ref mime) = head.mime {
        value += ",mime ";
        STANDARD.encode_string(mime.as_bytes(), &mut value);
    }

    if let Some(ref preview) = head.preview {
        value += ",preview ";
        STANDARD.encode_string(preview, &mut value);
    }

    HeaderValue::from_str(&value).unwrap()
}
