use ftl::*;

use schema::Snowflake;

use crate::{ctrl::auth::Authorization, web::routes::api::ApiError};

pub mod get;
pub mod patch;
//pub mod post;

pub async fn party_rooms(
    mut route: Route<crate::ServerState>,
    auth: Authorization,
    party_id: Snowflake,
) -> Response {
    match route.next().method_segment() {
        // POST /api/v1/party/1234/rooms
        //(&Method::POST, End) => post::post_room(route, auth, party_id).await,

        // GET /api/v1/party/1234/rooms
        (&Method::GET, End) => get::get(route, auth, party_id).await,

        // ANY /api/v1/party/1234/rooms/5678
        _ => match route.param::<Snowflake>() {
            Some(Ok(room_id)) => match route.next().method_segment() {
                // PATCH /api/v1/party/1234/room/5678
                (&Method::PATCH, End) => "Unimplemented".into_response(),

                _ => ApiError::not_found().into_response(),
            },
            _ => return ApiError::bad_request().into_response(),
        },
    }
}
