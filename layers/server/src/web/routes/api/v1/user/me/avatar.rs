use ftl::*;

use sdk::models::Snowflake;

use crate::{
    web::{auth::Authorization, routes::api::ApiError},
    ServerState,
};

pub async fn post_avatar(mut route: Route<ServerState>, auth: Authorization) -> Response {
    let file_id = match route.next().param::<Snowflake>() {
        Some(Ok(file_id)) => file_id,
        _ => return ApiError::bad_request().into_response(),
    };

    match crate::ctrl::user::me::avatar::process_avatar(route.state, auth.user_id, file_id).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
