use ftl::*;
use schema::Snowflake;

use crate::web::{
    auth::{authorize, Authorization},
    routes::api::ApiError,
};

pub mod get;
pub mod messages;
pub mod typing;
pub mod patch;

pub async fn room(mut route: Route<crate::ServerState>) -> Response {
    let auth = match authorize(&route).await {
        Ok(auth) => auth,
        Err(e) => return ApiError::err(e).into_response(),
    };

    // ANY /api/v1/room/1234
    match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => match route.next().method_segment() {
            (&Method::GET, End) => get::get_room(route, auth, room_id).await,
            (&Method::POST, Exact("typing")) => typing::trigger_typing(route, auth, room_id).await,

            (_, Exact("messages")) => messages::messages(route, auth, room_id).await,
            _ => ApiError::not_found().into_response(),
        },
        _ => ApiError::bad_request().into_response(),
    }
}
