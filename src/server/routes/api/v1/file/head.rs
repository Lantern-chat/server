use headers::{CacheControl, HeaderMapExt, HeaderValue};

use crate::{
    db::{schema::file::File, Snowflake},
    server::ftl::*,
};

pub async fn head(route: Route, file: File) -> impl Reply {
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
