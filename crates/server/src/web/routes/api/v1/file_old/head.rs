use headers::{CacheControl, HeaderMapExt, HeaderValue};

use ftl::*;

use db::{schema::file::File, Snowflake};

pub async fn head(route: Route<crate::ServerState>, file: File) -> Response {
    let mut res = Response::default();

    res.headers_mut().extend(
        super::TUS_HEADERS
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );

    let mut headers = res.headers_mut();

    headers.typed_insert(CacheControl::new().with_no_cache());

    headers.insert(
        "Upload-Length",
        HeaderValue::from_str(&file.size.to_string()).unwrap(),
    );

    headers.insert(
        "Upload-Offset",
        HeaderValue::from_str(&file.offset.to_string()).unwrap(),
    );

    res
}
