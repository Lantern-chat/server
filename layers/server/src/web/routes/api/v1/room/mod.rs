use super::*;

pub mod get;
pub mod messages;
pub mod patch;
pub mod threads;
pub mod typing;

pub fn room(mut route: Route<ServerState>, auth: MaybeAuth) -> RouteResult {
    let auth = auth.unwrap()?;

    // ANY /api/v1/room/1234
    match route.next().param::<Snowflake>() {
        Some(Ok(room_id)) => match route.next().method_segment() {
            (&Method::GET, End) => Ok(get::get_room(route, auth, room_id)),
            (&Method::POST, Exact("typing")) => Ok(typing::trigger_typing(route, auth, room_id)),

            (_, Exact("messages")) => messages::messages(route, auth, room_id),
            (_, Exact("threads")) => threads::threads(route, auth, room_id),
            _ => Err(Error::NotFound),
        },
        _ => Err(Error::BadRequest),
    }
}
