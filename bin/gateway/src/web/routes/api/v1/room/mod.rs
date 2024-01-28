use sdk::api::commands::room::{DeleteRoom, GetRoom, PatchRoom};

use super::*;

pub mod messages;
//pub mod threads;
pub mod typing;

pub fn room(mut route: Route<ServerState>, auth: MaybeAuth) -> RouteResult {
    let auth = auth.unwrap()?;

    // ANY /api/v1/room/1234
    match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => match route.next().method_segment() {
            (&Method::GET, End) => Ok(get(route, auth, room_id)),
            (&Method::PATCH, End) => Ok(patch(route, auth, room_id)),
            (&Method::DELETE, End) => Ok(delete(route, auth, room_id)),

            (&Method::POST, Exact("typing")) => Ok(typing::trigger_typing(route, auth, room_id)),

            (_, Exact("messages")) => messages::messages(route, auth, room_id),
            //(_, Exact("threads")) => threads::threads(route, auth, room_id),
            _ => err(CommonError::NotFound),
        },
        _ => err(CommonError::BadRequest),
    }
}

#[async_recursion]
pub async fn get(route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> ApiResult {
    Ok(RawMessage::authorized(auth, GetRoom { room_id }))
}

#[async_recursion]
pub async fn delete(route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> ApiResult {
    Ok(RawMessage::authorized(auth, DeleteRoom { room_id }))
}

#[async_recursion] #[rustfmt::skip]
pub async fn patch(mut route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> ApiResult {
    Ok(RawMessage::authorized(auth, PatchRoom {
        room_id,
        body: body::any(&mut route).await?,
    }))
}
