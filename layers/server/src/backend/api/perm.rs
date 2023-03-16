use db::pool::Client;

use crate::ServerState;

use sdk::models::*;

use crate::Error;

pub async fn get_cached_room_permissions_with_conn(
    state: &ServerState,
    db: &Client,
    user_id: Snowflake,
    room_id: Snowflake,
) -> Result<Permissions, Error> {
    if let Some(perm) = state.perm_cache.get(user_id, room_id).await {
        return Ok(perm.perms);
    }

    get_room_permissions(db, user_id, room_id).await
}

pub async fn get_cached_room_permissions(
    state: &ServerState,
    user_id: Snowflake,
    room_id: Snowflake,
) -> Result<Permissions, Error> {
    if let Some(perm) = state.perm_cache.get(user_id, room_id).await {
        return Ok(perm.perms);
    }

    let db = state.db.read.get().await?;

    get_room_permissions(&db, user_id, room_id).await
}

pub async fn get_room_permissions(
    db: &Client,
    user_id: Snowflake,
    room_id: Snowflake,
) -> Result<Permissions, Error> {
    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .cols(&[AggRoomPerms::Permissions1, AggRoomPerms::Permissions2])
                    .from_table::<AggRoomPerms>()
                    .and_where(AggRoomPerms::UserId.equals(Var::of(Users::Id)))
                    .and_where(AggRoomPerms::RoomId.equals(Var::of(Rooms::Id)))
            },
            &[&user_id, &room_id],
        )
        .await?;

    let mut perm = Permissions::empty();

    if let Some(row) = row {
        perm = Permissions::from_i64(row.try_get(0)?, row.try_get(1)?);

        if perm.contains(Permissions::ADMINISTRATOR) {
            perm = Permissions::all();
        }
    }

    Ok(perm)
}
