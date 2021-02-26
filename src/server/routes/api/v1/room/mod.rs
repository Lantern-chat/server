use http::{Method, StatusCode};

use crate::{
    db::Snowflake,
    server::{
        ftl::*,
        routes::api::auth::{authorize, Authorization},
    },
};

pub mod get;

pub mod messages;

pub async fn room(mut route: Route) -> impl Reply {
    let auth = match authorize(&route).await {
        Ok(auth) => auth,
        Err(err) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    // ANY /api/v1/room/1234
    match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => match route.next().method_segment() {
            (&Method::GET, End) => get::get_room(route, auth, room_id).await.into_response(),

            (_, Exact("messages")) => messages::messages(route, auth, room_id)
                .await
                .into_response(),

            _ => StatusCode::NOT_FOUND.into_response(),
        },
        _ => return StatusCode::BAD_REQUEST.into_response(),
    }
}
