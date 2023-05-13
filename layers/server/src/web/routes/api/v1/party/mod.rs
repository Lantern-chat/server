use super::*;

//pub mod delete;
pub mod get;
//pub mod patch;
pub mod post;

pub mod invites;
pub mod members;
pub mod rooms;
pub mod search;

pub fn party(mut route: Route<ServerState>, auth: MaybeAuth) -> RouteResult {
    let auth = auth.unwrap()?;

    match route.next().method_segment() {
        // POST /api/v1/party
        (&Method::POST, End) => Ok(post::post(route, auth)),

        // ANY /api/v1/party/1234
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(party_id)) => match route.next().method_segment() {
                // GET /api/v1/party/1234
                (&Method::GET, End) => Ok(get::get(route, auth, party_id)),

                // PATCH /api/v1/party/1234
                //(&Method::PATCH, End) => patch::patch(route, auth, party_id).await,

                // DELETE /api/v1/party/1234
                //(&Method::DELETE, End) => {
                //    delete::delete(route, auth, party_id).await
                //}

                // ANY /api/v1/party/1234/rooms
                (_, Exact("rooms")) => rooms::party_rooms(route, auth, party_id),

                // ANY /api/v1/party/1234/members
                (_, Exact("members")) => members::members(route, auth, party_id),

                (&Method::POST, Exact("search")) => Ok(search::search(route, auth, party_id)),

                _ => Err(Error::NotFound),
            },
            _ => Err(Error::BadRequest),
        },
        _ => Err(Error::NotFound),
    }
}
