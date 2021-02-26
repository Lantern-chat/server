use http::StatusCode;

use crate::{
    db::{schema::Room, Snowflake},
    server::{
        ftl::*,
        routes::api::{auth::Authorization, util::serde::SerializeFromIter},
    },
};

pub async fn get_rooms(route: Route, auth: Authorization, party_id: Snowflake) -> impl Reply {
    let rooms = match Room::of_party(&route.state.db, party_id).await {
        Ok(rooms) => rooms,
        Err(err) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    reply::json(&SerializeFromIter::new(rooms)).into_response()
}
