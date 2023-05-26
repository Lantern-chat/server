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

                (&Method::POST, Exact("search")) => Ok(search::search(route, auth, party_id)),

                _ => Err(Error::NotFound),
            },
            _ => Err(Error::BadRequest),
        },
        _ => Err(Error::NotFound),
    }
}

#[async_recursion]
pub async fn get(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> WebResult {
    Ok(WebResponse::new(
        crate::backend::api::party::get::get_party(route.state, auth, party_id).await?,
    ))
}

#[async_recursion]
pub async fn patch(mut route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> WebResult {
    let form = body::any(&mut route).await?;

    Ok(WebResponse::new(
        crate::backend::api::party::modify::modify_party(route.state, auth, party_id, form).await?,
    ))
}

#[async_recursion]
pub async fn post(mut route: Route<ServerState>, auth: Authorization) -> WebResult {
    let form = body::any(&mut route).await?;

    Ok(WebResponse::new(
        crate::backend::api::party::create::create_party(route.state, auth, form).await?,
    ))
}
