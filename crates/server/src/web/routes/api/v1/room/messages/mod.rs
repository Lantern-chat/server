use ftl::*;

use db::Snowflake;

use crate::routes::api::auth::Authorization;

pub mod delete;
pub mod get_many;
pub mod get_one;
pub mod patch;
pub mod post;

pub async fn messages(
    mut route: Route<crate::ServerState>,
    auth: Authorization,
    room_id: Snowflake,
) -> impl Reply {
    match route.next().method_segment() {
        // POST /api/v1/room/1234/messages
        (&Method::POST, End) => post::post(route, auth, room_id).await.into_response(),

        // GET /api/v1/room/1234/messages
        (&Method::GET, End) => get_many::get_many(route, auth, room_id)
            .await
            .into_response(),

        // ANY /api/v1/room/1234/messages/5678
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(msg_id)) => match route.method() {
                // GET /api/v1/room/1234/messages/5678
                &Method::GET => get_one::get_one(route, auth, room_id, msg_id)
                    .await
                    .into_response(),

                // PATCH /api/v1/room/1234/messages/5678
                &Method::PATCH => "Edit Message".into_response(),

                // DELETE /api/v1/room/1234/messages/5678
                &Method::DELETE => "Delete Message".into_response(),

                _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
            },
            Some(Err(_)) => StatusCode::BAD_REQUEST.into_response(),
            _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
        },

        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}