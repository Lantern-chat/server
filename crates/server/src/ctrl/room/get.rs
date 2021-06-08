use futures::{StreamExt, TryStreamExt};

use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, perm::get_cached_room_permissions, Error, SearchMode},
    ServerState,
};

use models::*;

pub async fn get_room(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
) -> Result<Room, Error> {
    let perms = get_cached_room_permissions(&state, auth.user_id, room_id).await?;

    if !perms.room.contains(RoomPermissions::VIEW_ROOM) {
        return Err(Error::NotFound);
    }

    let db = state.db.read.get().await?;

    unimplemented!()
}
