use futures::StreamExt;

use db::pool::Client;

use crate::State;

use sdk::models::*;

use crate::Error;

pub async fn get_cached_room_permissions_with_conn(
    state: &State,
    db: &Client,
    user_id: Snowflake,
    room_id: Snowflake,
) -> Result<Permission, Error> {
    if let Some(perm) = state.perm_cache.get(user_id, room_id).await {
        return Ok(perm.perm);
    }

    get_room_permissions(db, user_id, room_id).await
}

pub async fn get_cached_room_permissions(
    state: &State,
    user_id: Snowflake,
    room_id: Snowflake,
) -> Result<Permission, Error> {
    if let Some(perm) = state.perm_cache.get(user_id, room_id).await {
        return Ok(perm.perm);
    }

    let db = state.db.read.get().await?;

    get_room_permissions(&db, user_id, room_id).await
}

pub async fn get_room_permissions(
    db: &Client,
    user_id: Snowflake,
    room_id: Snowflake,
) -> Result<Permission, Error> {
    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .col(AggRoomPerms::Perms)
                    .from_table::<AggRoomPerms>()
                    .and_where(AggRoomPerms::UserId.equals(Var::of(Users::Id)))
                    .and_where(AggRoomPerms::RoomId.equals(Var::of(Rooms::Id)))
            },
            &[&user_id, &room_id],
        )
        .await?;

    let mut perm = Permission::empty();

    if let Some(row) = row {
        let raw_perm = row.try_get::<_, i64>(0)? as u64;

        if (raw_perm & Permission::PACKED_ADMIN) == Permission::PACKED_ADMIN {
            perm = Permission::ALL;
        } else {
            perm = Permission::unpack(raw_perm);
        }
    }

    Ok(perm)
}
