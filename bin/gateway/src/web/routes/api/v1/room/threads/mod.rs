use super::*;

pub mod get;

pub fn threads(mut route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> ApiResult {
    match route.next().method_segment() {
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(thread_id)) => match route.method() {
                //&Method::GET => Ok(get::get(route, auth, room_id, thread_id)),
                _ => Err(Error::MethodNotAllowed),
            },
            Some(Err(_)) => Err(Error::BadRequest),
            _ => Err(Error::MethodNotAllowed),
        },
        _ => Err(Error::MethodNotAllowed),
    }
}
