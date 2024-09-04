use sdk::api::commands::all::GetRoom;

use crate::prelude::*;

use sdk::models::*;

pub async fn get_room(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<GetRoom>,
) -> Result<FullRoom, Error> {
    crate::internal::get_rooms::get_room(state, auth, cmd.room_id.into()).await
}
