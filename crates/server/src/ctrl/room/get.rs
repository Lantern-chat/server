use futures::{StreamExt, TryStreamExt};

use schema::Snowflake;

use crate::{
    ctrl::{auth::Authorization, perm::get_cached_room_permissions_with_conn, Error, SearchMode},
    permission_cache::PermMute,
    ServerState,
};

use models::*;

// TODO: This
pub async fn get_room(state: ServerState, auth: Authorization, room_id: Snowflake) -> Result<Room, Error> {
    // TODO: Ensure the room permissions are cached after this
    let perms = match state.perm_cache.get(auth.user_id, room_id).await {
        Some(PermMute { perm, .. }) => {
            if !perm.room.contains(RoomPermissions::VIEW_ROOM) {
                return Err(Error::NotFound);
            }

            Some(perm)
        }
        None => None,
    };

    let db = state.db.read.get().await?;

    let row = if let Some(perms) = perms {
        db.query_opt_cached_typed(|| query(false), &[&room_id]).await
    } else {
        db.query_opt_cached_typed(|| query(true), &[&room_id, &auth.user_id])
            .await
    };

    match row {
        Ok(None) => Err(Error::NotFound),
        Err(e) => Err(e.into()),
        Ok(Some(row)) => Ok(unimplemented!()),
    }
}

use thorn::*;

fn query(perm: bool) -> impl AnyQuery {
    use schema::*;

    let query = Query::select().cols(&[
        Rooms::PartyId,
        Rooms::AvatarId,
        Rooms::ParentId,
        Rooms::SortOrder,
        Rooms::Flags,
        Rooms::Name,
        Rooms::Topic,
    ]);

    query
}
