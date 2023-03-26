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
    #[rustfmt::skip]
    let row = db.query_opt2(thorn::sql! {
        use schema::*;

        SELECT
            AggRoomPerms.Permissions1 AS @Permissions1,
            AggRoomPerms.Permissions2 AS @Permissions2
        FROM AggRoomPerms WHERE
            AggRoomPerms.UserId = #{&user_id => AggRoomPerms::UserId}
        AND AggRoomPerms.RoomId = #{&room_id => AggRoomPerms::RoomId}
    }?).await?;

    let mut perm = Permissions::empty();

    if let Some(row) = row {
        perm = Permissions::from_i64(row.permissions1()?, row.permissions2()?);

        if perm.contains(Permissions::ADMINISTRATOR) {
            perm = Permissions::all();
        }
    }

    Ok(perm)
}
