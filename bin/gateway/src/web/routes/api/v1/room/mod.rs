use super::*;

pub mod messages;
pub mod threads;
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
            (_, Exact("threads")) => threads::threads(route, auth, room_id),
            _ => Err(Error::NotFound),
        },
        _ => Err(Error::BadRequest),
    }
}

#[async_recursion]
pub async fn get(route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> WebResult {
    Ok(WebResponse::new(
        crate::backend::api::room::get::get_room(route.state, auth, room_id).await?,
    ))
}

#[async_recursion]
pub async fn delete(route: Route<crate::ServerState>, auth: Authorization, room_id: Snowflake) -> WebResult {
    Ok(WebResponse::new(
        crate::backend::api::room::remove::remove_room(route.state, auth, room_id).await?,
    ))
}

#[async_recursion]
pub async fn patch(mut route: Route<crate::ServerState>, auth: Authorization, room_id: Snowflake) -> WebResult {
    let form = body::any(&mut route).await?;

    Ok(WebResponse::new(
        crate::backend::api::room::modify::modify_room(route.state, auth, room_id, form).await?,
    ))
}
