use db::Snowflake;
use ftl::*;

use crate::web::auth::{authorize, Authorization};

pub mod get;
pub mod messages;
pub mod typing;

pub async fn room(mut route: Route<crate::ServerState>) -> impl Reply {
    let auth = match authorize(&route).await {
        Ok(auth) => auth,
        Err(_err) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    // ANY /api/v1/room/1234
    match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => match route.next().method_segment() {
            (&Method::GET, End) => get::get_room(route, auth, room_id).await.into_response(),
            (&Method::POST, Exact("typing")) => {
                typing::trigger_typing(route, auth, room_id).await.into_response()
            }

            (_, Exact("messages")) => messages::messages(route, auth, room_id).await.into_response(),
            _ => StatusCode::NOT_FOUND.into_response(),
        },
        _ => StatusCode::BAD_REQUEST.into_response(),
    }
}
