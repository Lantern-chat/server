use futures::StreamExt;

use schema::flags::RoomMemberFlags;
use sdk::models::{Permissions, Snowflake};

use crate::backend::cache::permission_cache::PermMute;
use crate::{Error, ServerState};

pub async fn refresh_room_perms(
    state: &ServerState,
    db: &db::pool::Object,
    user_id: Snowflake,
) -> Result<(), Error> {
    let stream = db
        .query_stream_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<AggRoomPerms>()
                    .cols(&[
                        AggRoomPerms::RoomId,
                        AggRoomPerms::Permissions1,
                        AggRoomPerms::Permissions2,
                    ])
                    .and_where(AggRoomPerms::UserId.equals(Var::of(Users::Id)))
            },
            &[&user_id],
        )
        .await?;

    let mut cache = Vec::new();
    let mut stream = std::pin::pin!(stream);

    while let Some(row) = stream.next().await {
        let row = row?;

        let room_id: Snowflake = row.try_get(0)?;

        cache.push((
            room_id,
            PermMute {
                perms: Permissions::from_i64(row.try_get(1)?, row.try_get(2)?),
                flags: RoomMemberFlags::empty(),
            },
        ));
    }

    log::trace!("Setting {} room permissions on user {user_id}", cache.len());
    state.perm_cache.batch_set(user_id, cache.into_iter()).await;

    Ok(())
}
