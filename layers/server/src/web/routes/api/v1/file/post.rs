use ftl::*;

use super::ApiResponse;
use crate::{Authorization, ServerState};

#[async_recursion]
pub async fn post(mut route: Route<ServerState>, auth: Authorization) -> ApiResponse {
    let body = body::any(&mut route).await?;

    let file_id = crate::backend::api::file::post::post_file(&route.state, auth.user_id, body).await?;

    let mut res = reply::json(file_id)
        .with_status(StatusCode::CREATED)
        .into_response();

    res.headers_mut().extend(super::tus_headers());

    res.headers_mut()
        .insert("Location", super::header_from_int(file_id.to_u64()));

    Ok(res)
}
