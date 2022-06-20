use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::{Authorization, Error};

pub mod delete;
pub mod get_many;
pub mod get_one;
pub mod patch;
pub mod post;

pub async fn messages(
    mut route: Route<crate::ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> ApiResponse {
    match route.next().method_segment() {
        // POST /api/v1/room/1234/messages
        (&Method::POST, End) => post::post(route, auth, room_id).await,

        // GET /api/v1/room/1234/messages
        (&Method::GET, End) => get_many::get_many(route, auth, room_id).await,

        // ANY /api/v1/room/1234/messages/5678
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(msg_id)) => match route.method() {
                // GET /api/v1/room/1234/messages/5678
                &Method::GET => get_one::get_one(route, auth, room_id, msg_id).await,

                // PATCH /api/v1/room/1234/messages/5678
                &Method::PATCH => patch::patch(route, auth, room_id, msg_id).await,

                // DELETE /api/v1/room/1234/messages/5678
                &Method::DELETE => delete::delete(route, auth, room_id, msg_id).await,

                _ => Err(Error::MethodNotAllowed),
            },
            Some(Err(_)) => Err(Error::BadRequest),
            _ => Err(Error::MethodNotAllowed),
        },
        _ => Err(Error::MethodNotAllowed),
    }
}
