use ftl::*;
use schema::Snowflake;

use crate::ServerState;

use crate::{ctrl::auth::Authorization, web::routes::api::ApiError};

pub mod get;

pub async fn threads(mut route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> Response {
    match route.next().method_segment() {
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(thread_id)) => match route.method() {
                &Method::GET => get::get(route, auth, room_id, thread_id).await,
                _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
            },
            Some(Err(_)) => ApiError::bad_request().into_response(),
            _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
        },
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}
