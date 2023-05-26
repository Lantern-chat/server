use super::*;

pub fn party_rooms(mut route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> RouteResult {
    match route.next().method_segment() {
        // POST /api/v1/party/1234/rooms
        (&Method::POST, End) => Ok(post(route, auth, party_id)),

        // GET /api/v1/party/1234/rooms
        (&Method::GET, End) => Ok(get(route, auth, party_id)),

        // // ANY /api/v1/party/1234/rooms/5678
        // _ => match route.param::<Snowflake>() {
        //     Some(Ok(room_id)) => match route.next().method_segment() {
        //         // PATCH /api/v1/party/1234/room/5678
        //         (&Method::PATCH, End) => Err(Error::Unimplemented),
        //
        //         _ => Err(Error::NotFound),
        //     },
        //     _ => Err(Error::NotFound),
        // },
        _ => Err(Error::NotFound),
    }
}

#[async_recursion]
pub async fn get(route: Route<crate::ServerState>, auth: Authorization, party_id: Snowflake) -> WebResult {
    use crate::backend::api::party::rooms::get::{get_rooms, RoomScope};

    Ok(WebResponse::stream(
        get_rooms(route.state, auth, RoomScope::Party(party_id)).await?,
    ))
}

#[async_recursion]
pub async fn post(mut route: Route<crate::ServerState>, auth: Authorization, party_id: Snowflake) -> WebResult {
    let form = body::any(&mut route).await?;

    Ok(WebResponse::new(
        crate::backend::api::party::rooms::create::create_room(route.state, auth, party_id, form).await?,
    ))
}
