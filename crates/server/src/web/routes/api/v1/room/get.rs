use ftl::*;

use db::Snowflake;

use crate::ctrl::room::get::get_room as get;
use crate::ctrl::Error;
use crate::web::auth::Authorization;

pub async fn get_room(
    route: Route<crate::ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> impl Reply {
    match crate::ctrl::room::get::get_room(route.state, auth, room_id).await {
        Ok(ref room) => reply::json(room).into_response(),
        Err(Error::NotFound) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            log::error!("Error getting room: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
