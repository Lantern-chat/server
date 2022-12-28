use super::*;

pub mod delete;
pub mod get_many;
pub mod get_one;
pub mod patch;
pub mod post;
pub mod reactions;

pub fn messages(mut route: Route<ServerState>, auth: Authorization, room_id: Snowflake) -> RouteResult {
    match route.next().method_segment() {
        // POST /api/v1/room/1234/messages
        (&Method::POST, End) => Ok(post::post(route, auth, room_id)),

        // GET /api/v1/room/1234/messages
        (&Method::GET, End) => Ok(get_many::get_many(route, auth, room_id)),

        // ANY /api/v1/room/1234/messages/5678
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(msg_id)) => match route.next().method_segment() {
                // GET /api/v1/room/1234/messages/5678
                (&Method::GET, End) => Ok(get_one::get_one(route, auth, room_id, msg_id)),

                // PATCH /api/v1/room/1234/messages/5678
                (&Method::PATCH, End) => Ok(patch::patch(route, auth, room_id, msg_id)),

                // DELETE /api/v1/room/1234/messages/5678
                (&Method::DELETE, End) => Ok(delete::delete(route, auth, room_id, msg_id)),

                (_, Exact("reactions")) => reactions::reactions(route, auth, room_id, msg_id),

                (_, End) => Err(Error::MethodNotAllowed),
                _ => Err(Error::NotFound),
            },
            Some(Err(_)) => Err(Error::BadRequest),
            _ => Err(Error::MethodNotAllowed),
        },
        _ => Err(Error::MethodNotAllowed),
    }
}
