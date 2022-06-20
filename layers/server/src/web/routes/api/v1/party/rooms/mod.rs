use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::{Authorization, Error};

pub mod get;
pub mod patch;
//pub mod post;

pub async fn party_rooms(
    mut route: Route<crate::ServerState>,
    auth: Authorization,
    party_id: Snowflake,
) -> ApiResponse {
    match route.next().method_segment() {
        // POST /api/v1/party/1234/rooms
        //(&Method::POST, End) => post::post_room(route, auth, party_id).await,

        // GET /api/v1/party/1234/rooms
        (&Method::GET, End) => get::get(route, auth, party_id).await,

        // ANY /api/v1/party/1234/rooms/5678
        _ => match route.param::<Snowflake>() {
            Some(Ok(room_id)) => match route.next().method_segment() {
                // PATCH /api/v1/party/1234/room/5678
                (&Method::PATCH, End) => Err(Error::Unimplemented),

                _ => Err(Error::NotFound),
            },
            _ => Err(Error::NotFound),
        },
    }
}
