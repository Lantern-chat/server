use headers::{CacheControl, HeaderMapExt, HeaderValue};

use super::*;
use crate::{backend::api::file::head::UploadHead, util::TupleClone};

#[async_recursion]
pub async fn head(route: Route<ServerState>, auth: Authorization, file_id: Snowflake) -> WebResult {
    let head = crate::backend::api::file::head::head(route.state, auth, file_id).await?;

    let mut res = Response::default();

    let headers = res.headers_mut();

    headers.extend(super::tus_headers());

    headers.insert("Upload-Metadata", encode_metadata(&head));

    headers.insert("Upload-Length", super::header_from_int(head.size));

    if head.size != head.offset {
        headers.insert("Upload-Offset", super::header_from_int(head.offset));
    }

    headers.typed_insert(CacheControl::new().with_no_store());

    Ok(res.into())
}

fn encode_metadata(head: &UploadHead) -> HeaderValue {
    let mut value = "filename ".to_owned();
    value += &base64::encode(head.name.as_bytes());

    if let Some(ref mime) = head.mime {
        value += ",mime ";
        value += &base64::encode(mime.as_bytes());
    }

    if let Some(ref preview) = head.preview {
        value += ",preview ";
        value += &base64::encode(preview);
    }

    HeaderValue::from_str(&value).unwrap()
}
