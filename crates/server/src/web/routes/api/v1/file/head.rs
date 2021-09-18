use ftl::*;
use headers::{CacheControl, HeaderMapExt, HeaderValue};
use models::Snowflake;

use crate::{
    ctrl::{file::head::UploadHead, Error},
    web::{auth::Authorization, routes::api::ApiError},
    ServerState,
};

pub async fn head(
    route: Route<ServerState>,
    auth: Authorization,
    file_id: Snowflake,
) -> Result<Response, Error> {
    let head = crate::ctrl::file::head::head(route.state, auth, file_id).await?;

    let mut res = Response::default();

    let headers = res.headers_mut();

    headers.extend(super::TUS_HEADERS.iter().map(|(k, v)| (k.clone(), v.clone())));

    headers.insert("Upload-Metadata", encode_metadata(&head));

    headers.insert("Upload-Length", HeaderValue::from_str(&head.size.to_string())?);

    if head.size != head.offset {
        headers.insert("Upload-Offset", HeaderValue::from_str(&head.offset.to_string())?);
    }

    headers.typed_insert(CacheControl::new().with_no_store());

    Ok(res)
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
