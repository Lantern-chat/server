use futures::{Stream, StreamExt, TryStreamExt};

use hashbrown::HashMap;

use db::Snowflake;

use models::*;

use crate::{
    ctrl::{auth::Authorization, Error},
    ServerState,
};

#[derive(Clone, Copy)]
struct RawOverwrite {
    id: Snowflake,
    deny: u64,
    allow: u64,
}

pub async fn get_rooms(
    state: ServerState,
    auth: Authorization,
    party_id: Snowflake,
) -> Result<Vec<Room>, Error> {
    let db = state.read_db().await;

    let owner_id_future = async {
        let row = db
            .query_one_cached_typed(
                || {
                    use db::schema::*;
                    use thorn::*;

                    Query::select()
                        .from_table::<Party>()
                        .col(Party::OwnerId)
                        .and_where(Party::Id.equals(Var::of(Party::Id)))
                },
                &[&party_id],
            )
            .await?;

        Ok::<Snowflake, Error>(row.try_get(0)?)
    };

    let rooms_future = async {
        let rows = db
            .query_cached_typed(
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
        let mut ids = Vec::new();

        for row in &rows {
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

            ids.push(room.id);
            rooms.insert(room.id, room);
        }

        let overwrites = db
            .query_stream_cached_typed(
                || {
                    use db::schema::*;
                    use thorn::*;

                    Query::select()
                        .from_table::<Overwrites>()
                        .cols(&[Overwrites::RoomId, Overwrites::Allow, Overwrites::Deny])
                        .expr(Builtin::coalesce((Overwrites::RoleId, Overwrites::UserId)))
                        .and_where(
                            Overwrites::RoomId.equals(Builtin::any(Var::of(SNOWFLAKE_ARRAY))),
                        )
                        .order_by(Overwrites::RoomId.ascending()) // group by room_id
                        .order_by(Overwrites::RoleId.ascending().nulls_last()) // sort role overwrites first
                },
                &[&ids],
            )
            .await?;

        let mut raw_overwrites = HashMap::<Snowflake, Vec<RawOverwrite>>::new();

        futures::pin_mut!(overwrites);
        while let Some(row) = overwrites.next().await {
            let row = row?;
            let room_id = row.try_get(0)?;

            if let Some(room) = rooms.get_mut(&room_id) {
                let raw = RawOverwrite {
                    allow: row.try_get::<_, i64>(1)? as u64,
                    deny: row.try_get::<_, i64>(2)? as u64,
                    id: row.try_get(3)?,
                };

                raw_overwrites.entry(room_id).or_default().push(raw);

                room.overwrites.push(Overwrite {
                    id: raw.id,
                    allow: Permission::unpack(raw.allow),
                    deny: Permission::unpack(raw.deny),
                });
            }
        }

        Ok::<_, Error>((rooms, raw_overwrites))
    };

    let roles_future = async {
        let stream = db
            .query_stream_cached_typed(
                || {
                    use db::schema::*;
                    use thorn::*;

                    Query::select()
                        .from(
                            Roles::left_join_table::<RoleMembers>()
                                .on(RoleMembers::RoleId.equals(Roles::Id)),
                        )
                        .and_where(RoleMembers::UserId.equals(Var::of(Users::Id)))
                        .cols(&[Roles::Id, Roles::Permissions])
                        .and_where(Roles::PartyId.equals(Var::of(Party::Id)))
                },
                &[&auth.user_id, &party_id],
            )
            .await?;

        let mut roles = HashMap::<Snowflake, u64>::new();

        futures::pin_mut!(stream);
        while let Some(row) = stream.next().await {
            let row = row?;
            roles.insert(row.try_get(0)?, row.try_get::<_, i64>(1)? as u64);
        }

        Ok(roles)
    };

    let (owner_id, (mut rooms, mut raw_overwrites), roles) =
        futures::future::try_join3(owner_id_future, rooms_future, roles_future).await?;

    // owner can view all rooms, so don't bother with this logic otherwise
    if auth.user_id != owner_id {
        // permissions for @everyone
        let everyone = roles.get(&party_id).unwrap().clone();

        // base party permissions for user
        let mut base = everyone;
        for role in roles.values() {
            base |= *role;
        }

        // if not admin, continue to filtering
        if (base & Permission::PACKED_ADMIN) != Permission::PACKED_ADMIN {
            rooms.retain(|_, room| {
                let mut room_perm = base;

                let mut allow = 0;
                let mut deny = 0;

                let mut user_overwrite = None;

                let raws = raw_overwrites.remove(&room.id).unwrap();

                // overwrites are sorted role-first
                for overwrite in &raws {
                    if roles.contains_key(&overwrite.id) {
                        deny |= overwrite.deny;
                        allow |= overwrite.allow;
                    } else if overwrite.id == auth.user_id {
                        user_overwrite = Some((overwrite.deny, overwrite.allow));
                        break;
                    }
                }

                room_perm &= !deny;
                room_perm |= allow;

                if let Some((user_deny, user_allow)) = user_overwrite {
                    room_perm &= !user_deny;
                    room_perm |= user_allow;
                }

                (room_perm & Permission::PACKED_VIEW_ROOM) == Permission::PACKED_VIEW_ROOM
            });
        }
    }

    Ok(rooms.into_iter().map(|(_, v)| v).collect())
}