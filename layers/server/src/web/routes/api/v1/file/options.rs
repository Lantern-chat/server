use ftl::*;
use headers::HeaderValue;

use crate::{
    web::{auth::Authorization, routes::api::ApiError},
    ServerState,
};
use schema::Snowflake;

pub async fn options(route: Route<ServerState>, auth: Authorization) -> Response {
    let options = match backend::api::file::options::file_options(&route.state, auth).await {
        Ok(options) => options,
        Err(e) => return ApiError::err(e).into_response(),
    };

    let mut res = reply::json(&options).into_response();

    let headers = res.headers_mut();

    headers.extend(super::TUS_HEADERS.iter().map(|(k, v)| (k.clone(), v.clone())));

    headers.insert("Upload-Quota-Used", super::header_from_int(options.quota_used));
    headers.insert("Upload-Quota-Total", super::header_from_int(options.quota_total));

    res
}
