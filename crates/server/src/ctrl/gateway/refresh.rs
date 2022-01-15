use futures::StreamExt;

use models::{Permission, Snowflake};

use crate::ctrl::Error;
use crate::permission_cache::PermMute;
use crate::ServerState;

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
                    .cols(&[AggRoomPerms::RoomId, AggRoomPerms::Perms])
                    .and_where(AggRoomPerms::UserId.equals(Var::of(Users::Id)))
            },
            &[&user_id],
        )
        .await?;

    let mut cache = Vec::new();

    futures::pin_mut!(stream);
    while let Some(row) = stream.next().await {
        let row = row?;

        let room_id: Snowflake = row.try_get(0)?;
        let perm: i64 = row.try_get(1)?;

        cache.push((
            room_id,
            PermMute {
                perm: Permission::unpack(perm as u64),
                muted: false,
            },
        ));
    }

    log::trace!("Setting {} room permissions on user {user_id}", cache.len());
    state.perm_cache.batch_set(user_id, cache.into_iter()).await;

    Ok(())
}
