use futures::{FutureExt, StreamExt, TryStreamExt};

use schema::Snowflake;

use crate::{
    ctrl::{
        auth::Authorization, perm::get_cached_room_permissions_with_conn,
        util::encrypted_asset::encrypt_snowflake_opt, Error, SearchMode,
    },
    permission_cache::PermMute,
    ServerState,
};

use sdk::models::*;

pub async fn get_room(state: ServerState, auth: Authorization, room_id: Snowflake) -> Result<Room, Error> {
    // TODO: Ensure the room permissions are cached after this
    let perms = match state.perm_cache.get(auth.user_id, room_id).await {
        Some(PermMute { perm, .. }) => {
            if !perm.room.contains(RoomPermissions::VIEW_ROOM) {
                return Err(Error::NotFound);
            }

            Some(perm)
        }
        None => None,
    };

    let db = state.db.read.get().await?;

    if let Some(perms) = perms {
        // simple fast-path for cached permissions AND without needing overwrites, so most connected users
        // NOTE: Having a cached permission implies they are in the party/DM of where that room exists
        if !perms.party.contains(PartyPermissions::MANAGE_PERMS) {
            return get_room_simple(state, db, room_id).await;
        }
    }

    return get_room_full(state, db, auth.user_id, room_id, perms)
        .boxed()
        .await;
}

/// Simple version for regular users with cached permissions saying they cannot view overwrites
/// which results in just a simple single lookup
async fn get_room_simple(
    state: ServerState,
    db: db::pool::Object,
    room_id: Snowflake,
) -> Result<Room, Error> {
    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .cols(&[
                        /*0*/ Rooms::PartyId,
                        /*1*/ Rooms::AvatarId,
                        /*2*/ Rooms::Name,
                        /*3*/ Rooms::Topic,
                        /*4*/ Rooms::Position,
                        /*5*/ Rooms::Flags,
                        /*6*/ Rooms::ParentId,
                    ])
                    .and_where(Rooms::Id.equals(Var::of(Rooms::Id)))
                    .and_where(Rooms::DeletedAt.is_null())
            },
            &[&room_id],
        )
        .await?;

    match row {
        None => Err(Error::NotFound),
        Some(row) => Ok(Room {
            id: room_id,
            party_id: row.try_get(0)?,
            avatar: encrypt_snowflake_opt(&state, row.try_get(1)?),
            name: row.try_get(2)?,
            topic: row.try_get(3)?,
            position: row.try_get(4)?,
            flags: RoomFlags::from_bits_truncate(row.try_get(5)?),
            rate_limit_per_user: None,
            parent_id: row.try_get(6)?,
            overwrites: Vec::new(),
        }),
    }
}

async fn get_room_full(
    state: ServerState,
    db: db::pool::Object,
    user_id: Snowflake,
    room_id: Snowflake,
    perms: Option<Permission>,
) -> Result<Room, Error> {
    let base_perm_future = async {
        let row = db
            .query_opt_cached_typed(
                || {
                    use schema::*;
                    use thorn::*;

                    let room_id_var = Var::at(Rooms::Id, 1);
                    let user_id_var = Var::at(Users::Id, 2);

                    Query::select()
                        .col(Party::OwnerId)
                        .expr(Builtin::array_agg_nonnull(Roles::Id))
                        .expr(Builtin::bit_or(Roles::Permissions))
                        .from(
                            // select rooms and everything else dervied
                            Rooms::inner_join(
                                Roles::left_join_table::<Party>()
                                    .on(Roles::PartyId.equals(Party::Id))
                                    .left_join_table::<RoleMembers>()
                                    .on(RoleMembers::RoleId.equals(Roles::Id)),
                            )
                            .on(Party::Id.equals(Rooms::PartyId)),
                        )
                        .and_where(Rooms::Id.equals(room_id_var))
                        .and_where(
                            RoleMembers::UserId
                                .equals(user_id_var)
                                .or(Roles::Id.equals(Party::Id)),
                        )
                },
                &[&room_id, &user_id],
            )
            .await?;

        Ok::<_, Error>(())
    };

    unimplemented!()
}
