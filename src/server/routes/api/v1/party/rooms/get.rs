use http::StatusCode;

use crate::{
    db::{schema::Room, Snowflake},
    server::{ftl::*, routes::api::auth::Authorization},
};

pub async fn get_rooms(route: Route, auth: Authorization, party_id: Snowflake) -> impl Reply {
    match Room::of_party(&route.state.db, party_id).await {
        Ok(rooms) => reply::json(&rooms).into_response(),
        Err(err) => {
            log::error!("Error getting party rooms: {}", err);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    }
}
