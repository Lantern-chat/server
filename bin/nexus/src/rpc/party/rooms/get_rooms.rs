use sdk::{api::commands::all::GetPartyRooms, models::*};

use crate::prelude::*;

use crate::internal::get_rooms::{get_rooms, RoomScope};

pub async fn get_party_rooms(
    state: ServerState,
    auth: Authorization,
    cmd: &Archived<GetPartyRooms>,
) -> Result<impl Stream<Item = Result<FullRoom, Error>>, Error> {
    get_rooms(state, auth, RoomScope::Party(cmd.party_id.into())).await
}
