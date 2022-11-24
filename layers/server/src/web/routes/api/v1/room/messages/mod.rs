use ftl::*;

use futures::FutureExt;
use schema::Snowflake;

use super::ApiResponse;
use crate::{Authorization, Error};

pub mod delete;
pub mod get_many;
pub mod get_one;
pub mod patch;
pub mod post;
pub mod reactions;

pub async fn messages(
    mut route: Route<crate::ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> ApiResponse {
    match route.next().method_segment() {
        // POST /api/v1/room/1234/messages
        (&Method::POST, End) => post::post(route, auth, room_id).boxed().await,

        // GET /api/v1/room/1234/messages
        (&Method::GET, End) => get_many::get_many(route, auth, room_id).boxed().await,

        // ANY /api/v1/room/1234/messages/5678
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(msg_id)) => match route.next().method_segment() {
                // GET /api/v1/room/1234/messages/5678
                (&Method::GET, End) => get_one::get_one(route, auth, room_id, msg_id).boxed().await,

                // PATCH /api/v1/room/1234/messages/5678
                (&Method::PATCH, End) => patch::patch(route, auth, room_id, msg_id).boxed().await,

                // DELETE /api/v1/room/1234/messages/5678
                (&Method::DELETE, End) => delete::delete(route, auth, room_id, msg_id).boxed().await,

                (_, Exact("reactions")) => reactions::reactions(route, auth, room_id, msg_id).await,

                (_, End) => Err(Error::MethodNotAllowed),
                _ => Err(Error::NotFound),
            },
            Some(Err(_)) => Err(Error::BadRequest),
            _ => Err(Error::MethodNotAllowed),
        },
        _ => Err(Error::MethodNotAllowed),
    }
}
