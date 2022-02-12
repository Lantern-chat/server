use ftl::*;

use schema::Snowflake;

use crate::{
    ctrl::{file::post::FilePostBody, Error},
    web::{auth::Authorization, routes::api::ApiError},
    ServerState,
};

pub async fn post(mut route: Route<ServerState>, auth: Authorization) -> Response {
    let body = match body::any(&mut route).await {
        Ok(body) => body,
        Err(e) => return ApiError::err(e.into()).into_response(),
    };

    match crate::ctrl::file::post::post_file(route.state.clone(), auth.user_id, body).await {
        Err(e) => ApiError::err(e).into_response(),
        Ok(file_id) => {
            let mut res = reply::json(&file_id)
                .with_status(StatusCode::CREATED)
                .into_response();

            res.headers_mut()
                .extend(super::TUS_HEADERS.iter().map(|(k, v)| (k.clone(), v.clone())));

            res.headers_mut()
                .insert("Location", super::header_from_int(file_id.to_u64()));

            res
        }
    }
}

use http::header::HeaderValue;
use std::str::FromStr;
