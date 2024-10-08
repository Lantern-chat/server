use futures::StreamExt;

use schema::flags::RoomMemberFlags;
use sdk::models::Permissions;

use crate::backend::cache::permission_cache::PermMute;
use crate::prelude::*;

pub async fn refresh_room_perms(state: &ServerState, db: &db::Object, user_id: UserId) -> Result<(), Error> {
    #[rustfmt::skip]
    let stream = db.query_stream2(schema::sql! {
        const_assert!(!Columns::IS_DYNAMIC);

        SELECT
            AggRoomPerms.Id             AS @RoomId,
            AggRoomPerms.Permissions1   AS @Permissions1,
            AggRoomPerms.Permissions2   AS @Permissions2
        FROM AggRoomPerms
        WHERE AggRoomPerms.UserId = #{&user_id as AggRoomPerms::UserId}
    }).await?;

    let mut cache = Vec::new();
    let mut stream = std::pin::pin!(stream);

    while let Some(row) = stream.next().await {
        let row = row?;
        cache.push((
            row.room_id()?,
            PermMute {
                perms: Permissions::from_i64(row.permissions1()?, row.permissions2()?),
                flags: RoomMemberFlags::empty(),
            },
        ));
    }

    log::trace!("Setting {} room permissions on user {user_id}", cache.len());
    state.perm_cache.batch_set(user_id, cache.into_iter()).await;

    Ok(())
}
