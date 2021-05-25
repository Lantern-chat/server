use futures::{Stream, StreamExt};

use hashbrown::HashMap;

use db::Snowflake;

use models::*;

use crate::{
    ctrl::{auth::Authorization, Error},
    ServerState,
};

pub async fn get_rooms(
    state: ServerState,
    auth: Authorization,
    party_id: Snowflake,
) -> Result<Vec<Room>, Error> {
    let db = state.read_db().await;

    let stream = db
        .query_stream_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::select()
                    .from(
                        Rooms::left_join_table::<PartyMember>()
                            .on(Rooms::PartyId.equals(PartyMember::PartyId)),
                    )
                    .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
                    .cols(&[
                        Rooms::Id,
                        Rooms::Name,
                        Rooms::Topic,
                        Rooms::Flags,
                        Rooms::AvatarId,
                        Rooms::SortOrder,
                        Rooms::ParentId,
                    ])
                    .and_where(Rooms::DeletedAt.is_null())
                    .and_where(Rooms::PartyId.equals(Var::of(Party::Id)))
            },
            &[&auth.user_id, &party_id],
        )
        .await?;

    let mut rooms = HashMap::new();

    futures::pin_mut!(stream);
    while let Some(row) = stream.next().await {
        let row = row?;

        let room = Room {
            id: row.try_get(0)?,
            party_id: Some(party_id),
            name: row.try_get(1)?,
            topic: row.try_get(2)?,
            flags: RoomFlags::from_bits_truncate(row.try_get(3)?),
            icon_id: row.try_get(4)?,
            sort_order: row.try_get(5)?,
            rate_limit_per_user: None,
            parent_id: row.try_get(6)?,
            overwrites: Vec::new(),
        };

        rooms.insert(room.id, room);
    }

    let ids: Vec<Snowflake> = rooms.keys().copied().collect();

    let overwrites = db
        .query_stream_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Overwrites>()
                    .cols(&[Overwrites::RoomId, Overwrites::Allow, Overwrites::Deny])
                    .expr(Builtin::coalesce((Overwrites::RoleId, Overwrites::UserId)))
                    .and_where(Overwrites::RoomId.equals(Builtin::any(Var::of(SNOWFLAKE_ARRAY))))
            },
            &[&ids],
        )
        .await?;

    futures::pin_mut!(overwrites);
    while let Some(row) = overwrites.next().await {
        let row = row?;

        let overwrite = Overwrite {
            allow: Permission::unpack(row.try_get::<_, i64>(1)? as u64),
            deny: Permission::unpack(row.try_get::<_, i64>(2)? as u64),
            id: row.try_get(3)?,
        };

        let room_id = row.try_get(0)?;

        match rooms.get_mut(&room_id) {
            Some(room) => room.overwrites.push(overwrite),
            None => unreachable!(),
        }
    }

    Ok(rooms.into_iter().map(|(_, v)| v).collect())
}
