use ftl::*;

use db::{schema::Room, Snowflake};

use crate::web::auth::Authorization;

pub async fn get_room(
    route: Route<crate::ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> impl Reply {
    match Room::find(&route.state.db, room_id).await {
        Ok(Some(ref room)) => reply::json(room).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            log::error!("Error getting room: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
