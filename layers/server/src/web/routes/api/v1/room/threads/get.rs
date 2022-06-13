use ftl::*;
use schema::Snowflake;

use crate::ServerState;

use crate::{ctrl::auth::Authorization, web::routes::api::ApiError};

pub async fn get(
    route: Route<ServerState>,
    auth: Authorization,
    room_id: Snowflake,
    thread_id: Snowflake,
) -> Response {
    match crate::ctrl::room::threads::get::get_thread(route.state, auth, room_id, thread_id).await {
        Ok(ref thread) => reply::json(thread).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
