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

use models::*;

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

    return get_room_full(state, db, room_id).boxed().await;
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
                        /*4*/ Rooms::SortOrder,
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
            sort_order: row.try_get(4)?,
            flags: RoomFlags::from_bits_truncate(row.try_get(5)?),
            rate_limit_per_user: None,
            parent_id: row.try_get(6)?,
            overwrites: Vec::new(),
        }),
    }
}

async fn get_room_full(state: ServerState, db: db::pool::Object, room_id: Snowflake) -> Result<Room, Error> {
    unimplemented!()
}
