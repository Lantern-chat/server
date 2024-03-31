use sdk::api::commands::party::{CreateParty, GetParty, PatchParty};

use super::*;

pub mod invites;
pub mod members;
pub mod rooms;
pub mod search;

pub fn party(mut route: Route<ServerState>, auth: MaybeAuth) -> RouteResult {
    let auth = auth.unwrap()?;

    match route.next().method_segment() {
        // POST /api/v1/party
        (&Method::POST, End) => Ok(post(route, auth)),

        // ANY /api/v1/party/1234
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(party_id)) => match route.next().method_segment() {
                // GET /api/v1/party/1234
                (&Method::GET, End) => Ok(get(route, auth, party_id)),

                // PATCH /api/v1/party/1234
                (&Method::PATCH, End) => Ok(patch(route, auth, party_id)),

                // DELETE /api/v1/party/1234
                //(&Method::DELETE, End) => {
                //    delete::delete(route, auth, party_id).await
                //}

                // ANY /api/v1/party/1234/rooms
                (_, Exact("rooms")) => rooms::party_rooms(route, auth, party_id),

                // ANY /api/v1/party/1234/members
                (_, Exact("members")) => members::members(route, auth, party_id),

                (m, Exact("search")) if m == Method::POST || m == *crate::web::METHOD_QUERY => {
                    Ok(search::search(route, auth, party_id))
                }

                _ => Err(Error::NotFoundSignaling),
            },
            _ => Err(Error::BadRequest),
        },
        _ => Err(Error::NotFoundSignaling),
    }
}

#[async_recursion]
pub async fn get(route: Route<ServerState>, _auth: Authorization, party_id: Snowflake) -> ApiResult {
    Ok(Procedure::from(GetParty { party_id }))
}

#[async_recursion] #[rustfmt::skip]
pub async fn patch(mut route: Route<ServerState>, _auth: Authorization, party_id: Snowflake) -> ApiResult {
    Ok(Procedure::from(PatchParty { party_id, body: body::any(&mut route).await? }))
}

#[async_recursion] #[rustfmt::skip]
pub async fn post(mut route: Route<ServerState>, _auth: Authorization) -> ApiResult {
    Ok(Procedure::from(CreateParty { body: body::any(&mut route).await? }))
}
