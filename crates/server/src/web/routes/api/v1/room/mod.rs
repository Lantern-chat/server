use db::Snowflake;
use ftl::*;

use crate::ctrl::perm::get_room_permissions;
use crate::web::auth::{authorize, Authorization};

pub mod get;

pub mod messages;

pub async fn room(mut route: Route<crate::ServerState>) -> impl Reply {
    let auth = match authorize(&route).await {
        Ok(auth) => auth,
        Err(_err) => return StatusCode::UNAUTHORIZED.into_response(),
    };

    // ANY /api/v1/room/1234
    match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => match route.next().method_segment() {
            (&Method::GET, End) => get::get_room(route, auth, room_id).await.into_response(),

            (&Method::GET, Exact("test")) => test(route.state, auth, room_id).await.into_response(),

            (_, Exact("messages")) => messages::messages(route, auth, room_id)
                .await
                .into_response(),
            _ => StatusCode::NOT_FOUND.into_response(),
        },
        _ => StatusCode::BAD_REQUEST.into_response(),
    }
}

use crate::web::routes::api::ApiError;
use crate::ServerState;

async fn test(state: ServerState, auth: Authorization, room_id: Snowflake) -> impl Reply {
    let db = state.read_db().await;

    match get_room_permissions(&db, auth.user_id, room_id).await {
        Ok(ref perm) => reply::json(perm).into_response(),
        Err(e) => {
            log::error!("Error getting room permissions");
            ApiError::err(e).into_response()
        }
    }
}
