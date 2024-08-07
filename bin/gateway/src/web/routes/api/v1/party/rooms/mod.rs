use sdk::api::commands::party::{CreateRoom, GetPartyRooms};

use super::*;

pub fn party_rooms(mut route: Route<ServerState>, auth: Authorization, party_id: PartyId) -> RouteResult {
    match route.next().method_segment() {
        // POST /api/v1/party/1234/rooms
        (&Method::POST, End) => Ok(post(route, auth, party_id)),

        // GET /api/v1/party/1234/rooms
        (&Method::GET, End) => Ok(get(route, auth, party_id)),

        // // ANY /api/v1/party/1234/rooms/5678
        // _ => match route.param::<RoomId>() {
        //     Some(Ok(room_id)) => match route.next().method_segment() {
        //         // PATCH /api/v1/party/1234/room/5678
        //         (&Method::PATCH, End) => Err(Error::Unimplemented),
        //
        //         _ => Err(Error::NotFound),
        //     },
        //     _ => Err(Error::NotFound),
        // },
        _ => Err(Error::NotFoundSignaling),
    }
}

#[async_recursion]
pub async fn get(route: Route<ServerState>, _auth: Authorization, party_id: PartyId) -> ApiResult {
    Ok(Procedure::from(GetPartyRooms { party_id }))
}

#[async_recursion] #[rustfmt::skip]
pub async fn post(mut route: Route<ServerState>, _auth: Authorization, party_id: PartyId) -> ApiResult {
    Ok(Procedure::from(CreateRoom { party_id, body: body::any(&mut route).await? }))
}
