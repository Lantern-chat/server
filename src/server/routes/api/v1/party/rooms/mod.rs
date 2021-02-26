use http::{Method, StatusCode};

use crate::{
    db::Snowflake,
    server::{ftl::*, routes::api::auth::Authorization},
};

pub mod get;
pub mod patch;
pub mod post;

pub async fn party_rooms(mut route: Route, auth: Authorization, party_id: Snowflake) -> impl Reply {
    match route.next().method_segment() {
        // POST /api/v1/party/1234/rooms
        (&Method::POST, End) => "".into_response(),

        // GET /api/v1/party/1234/rooms
        (&Method::GET, End) => get::get_rooms(route, auth, party_id).await.into_response(),

        // ANY /api/v1/party/1234/rooms/5678
        _ => match route.param::<Snowflake>() {
            Some(Ok(room_id)) => match route.next().method_segment() {
                // PATCH /api/v1/party/1234/room/5678
                (&Method::PATCH, End) => "".into_response(),

                _ => StatusCode::NOT_FOUND.into_response(),
            },
            _ => return StatusCode::BAD_REQUEST.into_response(),
        },
    }
}
