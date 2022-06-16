use ftl::*;

use super::ApiResponse;
use crate::{Authorization, ServerState};

pub async fn options(route: Route<ServerState>, auth: Authorization) -> ApiResponse {
    let options = crate::backend::api::file::options::file_options(&route.state, auth).await?;

    let mut res = reply::json(options).into_response();

    let headers = res.headers_mut();

    headers.extend(super::tus_headers());

    headers.insert("Upload-Quota-Used", super::header_from_int(options.quota_used));
    headers.insert("Upload-Quota-Total", super::header_from_int(options.quota_total));

    Ok(res)
}
