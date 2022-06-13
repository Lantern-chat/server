use ftl::*;

use schema::Snowflake;

use crate::{ctrl::auth::Authorization, web::routes::api::ApiError, ServerState};

pub async fn get_one(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> Response {
    match crate::ctrl::room::messages::get_one::get_one(route.state, auth, room_id, msg_id).await {
        Ok(ref msg) => reply::json(msg).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
