use futures::{StreamExt, TryStreamExt};

use schema::Snowflake;

use crate::{
    ctrl::{auth::Authorization, perm::get_cached_room_permissions_with_conn, Error, SearchMode},
    ServerState,
};

use models::*;

pub async fn get_room(state: ServerState, auth: Authorization, room_id: Snowflake) -> Result<Room, Error> {
    let had_perms = if let Some(perms) = state.perm_cache.get(auth.user_id, room_id).await {
        if !perms.perm.room.contains(RoomPermissions::VIEW_ROOM) {
            return Err(Error::NotFound);
        }
        true
    } else {
        false
    };

    let db = state.db.read.get().await?;

    let row = if had_perms {
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
