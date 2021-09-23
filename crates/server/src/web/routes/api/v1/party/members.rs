use ftl::*;
use schema::Snowflake;

use crate::{ctrl::auth::Authorization, ctrl::Error, web::routes::api::ApiError, ServerState};

pub async fn get_members(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> Response {
    match crate::ctrl::party::members::get_members(route.state, party_id, auth.user_id).await {
        Ok(stream) => reply::json_stream(stream).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
