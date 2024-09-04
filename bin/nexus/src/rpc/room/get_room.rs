use futures::StreamExt;

use crate::prelude::*;

use crate::rpc::party::rooms::get_rooms::{get_rooms, RoomScope};

use sdk::models::*;

pub async fn get_room(state: ServerState, auth: Authorization, room_id: RoomId) -> Result<FullRoom, Error> {
    let stream = get_rooms(state, auth, RoomScope::Room(room_id)).await?;

    match std::pin::pin!(stream).next().await {
        Some(res) => res,
        None => Err(Error::NotFound),
    }
}
