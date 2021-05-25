use ftl::*;

use db::Snowflake;

use crate::ctrl::auth::Authorization;

pub async fn get_rooms(
    route: Route<crate::ServerState>,
    auth: Authorization,
    party_id: Snowflake,
) -> impl Reply {
    //match Room::of_party(&route.state.db, party_id).await {
    //    Ok(rooms) => reply::json(&rooms).into_response(),
    //    Err(err) => {
    //        log::error!("Error getting party rooms: {}", err);
    //        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    //    }
    //}
}
