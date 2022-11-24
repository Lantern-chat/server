use ftl::*;
use futures::FutureExt;
use schema::Snowflake;

use super::ApiResponse;
use crate::{Error, ServerState};

pub mod get;
pub mod messages;
pub mod patch;
pub mod threads;
pub mod typing;

pub async fn room(mut route: Route<ServerState>) -> ApiResponse {
    let auth = crate::web::auth::authorize(&route).await?;

    // ANY /api/v1/room/1234
    match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => match route.next().method_segment() {
            (&Method::GET, End) => get::get_room(route, auth, room_id).boxed().await,
            (&Method::POST, Exact("typing")) => typing::trigger_typing(route, auth, room_id).boxed().await,

            (_, Exact("messages")) => messages::messages(route, auth, room_id).await,
            (_, Exact("threads")) => threads::threads(route, auth, room_id).await,
            _ => Err(Error::NotFound),
        },
        _ => Err(Error::BadRequest),
    }
}
