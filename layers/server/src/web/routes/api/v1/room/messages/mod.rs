use super::*;

pub mod delete;
pub mod get_many;
pub mod get_one;
pub mod patch;
pub mod pin;
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

                // ANY /api/v1/room/1234/messages/5678/reactions
                (_, Exact("reactions")) => reactions::reactions(route, auth, room_id, msg_id),

                // PUT | DELETE /api/v1/room/1234/messages/5678/star/9012
                (&Method::PUT | &Method::DELETE, Exact("pin")) => match route.next().param::<Snowflake>() {
                    Some(Ok(pin_id)) => match route.method() {
                        &Method::PUT => Ok(pin::pin_message(route, auth, msg_id, pin_id)),
                        &Method::DELETE => Ok(pin::unpin_message(route, auth, msg_id, pin_id)),
                        _ => unreachable!(),
                    },
                    _ => Err(Error::BadRequest),
                },

                // PUT /api/v1/room/1234/messages/5678/star
                (&Method::PUT, Exact("star")) => Ok(pin::star_message(route, auth, msg_id)),

                // DELETE /api/v1/room/1234/messages/5678/star
                (&Method::DELETE, Exact("star")) => Ok(pin::unstar_message(route, auth, msg_id)),

                (_, End) => Err(Error::MethodNotAllowed),
                _ => Err(Error::NotFound),
            },
            Some(Err(_)) => Err(Error::BadRequest),
            _ => Err(Error::MethodNotAllowed),
        },
        _ => Err(Error::MethodNotAllowed),
    }
}
