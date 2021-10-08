use ftl::*;

use schema::Snowflake;

use crate::{ctrl::auth::Authorization, web::routes::api::ApiError, ServerState};

pub async fn delete(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> Response {
    match crate::ctrl::room::messages::delete::delete_msg(route.state, auth, room_id, msg_id).await {
        Ok(res) => res.into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
