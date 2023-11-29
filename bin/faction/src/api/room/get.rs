use futures::StreamExt;

use schema::Snowflake;

use crate::backend::api::party::rooms::get::{get_rooms, RoomScope};
use crate::{Authorization, Error, ServerState};

use sdk::models::*;

pub async fn get_room(state: ServerState, auth: Authorization, room_id: Snowflake) -> Result<FullRoom, Error> {
    let stream = get_rooms(state, auth, RoomScope::Room(room_id)).await?;

    match std::pin::pin!(stream).next().await {
        Some(res) => res,
        None => Err(Error::NotFound),
    }
}
