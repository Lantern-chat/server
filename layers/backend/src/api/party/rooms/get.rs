use futures::{Stream, StreamExt, TryStreamExt};

use hashbrown::HashMap;

use schema::Snowflake;

use sdk::models::*;

use crate::{
    api::{auth::Authorization, SearchMode},
    util::encrypted_asset::encrypt_snowflake_opt,
    Error, State,
};

#[derive(Clone, Copy)]
struct RawOverwrite {
    room_id: Snowflake,
    user_id: Option<Snowflake>,
    role_id: Option<Snowflake>,
    deny: u64,
    allow: u64,
}

impl From<RawOverwrite> for Overwrite {
    fn from(raw: RawOverwrite) -> Self {
        Overwrite {
            id: raw.user_id.or(raw.role_id).expect("No valid ID given"),
            allow: Permission::unpack(raw.allow),
            deny: Permission::unpack(raw.deny),
        }
    }
}

pub async fn get_rooms(state: State, auth: Authorization, party_id: Snowflake) -> Result<Vec<Room>, Error> {
    let db = state.db.read.get().await?;

    let base_perm_future = async {
        let row = db
            .query_one_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    Query::select()
                        .col(Party::OwnerId)
                        .expr(Builtin::array_agg_nonnull(Roles::Id))
                        .expr(Builtin::bit_or(Roles::Permissions))
                        .from(
                            Roles::left_join_table::<Party>()
                                .on(Roles::PartyId.equals(Party::Id))
                                .left_join_table::<RoleMembers>()
                                .on(RoleMembers::RoleId.equals(Roles::Id)),
                        )
                        .and_where(Party::Id.equals(Var::of(Party::Id)))
                        .and_where(
                            // @user and @everyone roles
                            RoleMembers::UserId
                                .equals(Var::of(Users::Id))
                                .or(Roles::Id.equals(Party::Id)),
                        )
                        .group_by(Party::OwnerId)
                },
                &[&party_id, &auth.user_id],
            )
            .await?;

        let owner_id: Snowflake = row.try_get(0)?;
        let role_ids: Vec<Snowflake> = row.try_get(1)?;

        let permissions = if owner_id == auth.user_id {
            Permission::ALL
        } else {
            Permission::unpack(row.try_get::<_, i64>(2)? as u64)
        };

        Ok((permissions, role_ids))
    };

    let rooms_future = async {
        let rows = db
            .query_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    Query::select()
                        .from(
                            Rooms::inner_join_table::<PartyMember>()
                                .on(Rooms::PartyId.equals(PartyMember::PartyId)),
                        )
                        .and_where(PartyMember::UserId.equals(Var::of(Users::Id)))
                        .cols(&[
                            Rooms::Id,
                            Rooms::Name,
                            Rooms::Topic,
                            Rooms::Flags,
                            Rooms::AvatarId,
                            Rooms::Position,
                            Rooms::ParentId,
                        ])
                        .and_where(Rooms::DeletedAt.is_null())
                        .and_where(Rooms::PartyId.equals(Var::of(Party::Id)))
                },
                &[&auth.user_id, &party_id],
            )
            .await?;

        let mut rooms = HashMap::with_capacity(rows.len());

        for row in &rows {
            let room = Room {
                id: row.try_get(0)?,
                party_id: Some(party_id),
                name: row.try_get(1)?,
                topic: row.try_get(2)?,
                flags: RoomFlags::from_bits_truncate(row.try_get(3)?),
                avatar: encrypt_snowflake_opt(&state, row.try_get(4)?),
                position: row.try_get(5)?,
                rate_limit_per_user: None,
                parent_id: row.try_get(6)?,
                overwrites: Vec::new(),
            };

            rooms.insert(room.id, room);
        }

        Ok(rooms)
    };

    let overwrites_future = async {
        let rows = db
            .query_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    Query::select()
                        .cols(&[
                            Overwrites::RoomId,
                            Overwrites::Allow,
                            Overwrites::Deny,
                            Overwrites::RoleId,
                            Overwrites::UserId,
                        ])
                        .order_by(Overwrites::RoomId.ascending()) // group by room_id
                        .order_by(Overwrites::RoleId.ascending().nulls_last()) // sort role overwrites first
                        .from(Overwrites::left_join_table::<Rooms>().on(Overwrites::RoomId.equals(Rooms::Id)))
                        .and_where(Rooms::PartyId.equals(Var::of(Party::Id)))
                },
                &[&party_id],
            )
            .await?;

        let mut raw_overwrites = Vec::with_capacity(rows.len());

        for row in rows {
            raw_overwrites.push(RawOverwrite {
                room_id: row.try_get(0)?,
                allow: row.try_get::<_, i64>(1)? as u64,
                deny: row.try_get::<_, i64>(2)? as u64,
                role_id: row.try_get(3)?,
                user_id: row.try_get(4)?,
            });
        }

        Ok::<_, Error>(raw_overwrites)
    };

    let ((base_perm, roles), mut rooms, raw_overwrites) =
        tokio::try_join!(base_perm_future, rooms_future, overwrites_future)?;

    // iterate over raw overwrites and accumulate them in the correct room
    // this lazily fetches different rooms only when the room_id changes,
    // rather than a hashtable look up each iteration. Just an opportunistic thing for free.
    let mut raw_overwrites = raw_overwrites.into_iter();
    if let Some(raw) = raw_overwrites.next() {
        let mut room = rooms.get_mut(&raw.room_id).unwrap();

        room.overwrites.push(raw.into());

        for raw in raw_overwrites {
            if room.id != raw.room_id {
                room = rooms.get_mut(&raw.room_id).unwrap();
            }

            room.overwrites.push(raw.into());
        }
    }

    if !base_perm.is_admin() {
        rooms.retain(|_, room| {
            let room_perm = base_perm.compute_overwrites(&room.overwrites, &roles, auth.user_id);

            let can_view = room_perm.contains(RoomPermissions::VIEW_ROOM);

            // TODO: Determine the usefulness of hiding stuff
            //// Do not display overwrites to users without the permission to manage permissions
            //if can_view && !room_perm.contains(PartyPermissions::MANAGE_PERMS) {
            //    room.overwrites.clear();
            //}

            can_view
        });
    }

    Ok(rooms.into_iter().map(|(_, v)| v).collect())
}
