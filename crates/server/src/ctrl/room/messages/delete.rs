use schema::{Snowflake, SnowflakeExt};
use smol_str::SmolStr;

use crate::{
    ctrl::{auth::Authorization, perm::get_cached_room_permissions, Error, SearchMode},
    ServerState,
};

use sdk::models::*;

pub async fn delete_msg(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> Result<(), Error> {
    let perm = state.perm_cache.get(auth.user_id, room_id).await;

    if let Some(ref perm) = perm {
        // user cannot view channel at all
        if !perm.perm.room.contains(RoomPermissions::READ_MESSAGES) {
            return Err(Error::Unauthorized);
        }
    }

    let db = state.db.write.get().await?;

    use arrayvec::ArrayVec;

    let mut params: ArrayVec<&(dyn pg::ToSql + Sync), 3> =
        ArrayVec::from([&msg_id as _, &auth.user_id as _, &room_id as _]);

    let query = match perm {
        Some(perm) => {
            if perm.perm.room.contains(RoomPermissions::MANAGE_MESSAGES) {
                params.truncate(1); // only needs msg_id
                db.prepare_cached_typed(|| delete_without_perms()).await
            } else {
                params.truncate(2); // needs msg_id, user_id
                db.prepare_cached_typed(|| delete_if_own()).await
            }
        }
        // needs all three
        None => db.prepare_cached_typed(|| delete_with_perms()).await,
    };

    match db.execute(&query?, &params).await {
        Ok(1) => Ok(()),
        Ok(_) => Err(Error::Unauthorized),
        Err(e) => Err(e.into()),
    }
}

use thorn::*;

const DELETED_FLAG: Literal = Literal::Int2(MessageFlags::DELETED.bits());

///!!! NOTE: All parameters of these are sorted as: [msg_id, user_id, room_id]

fn delete_without_perms() -> impl AnyQuery {
    use schema::*;

    Query::update()
        .table::<Messages>()
        .set(Messages::Flags, Messages::Flags.bit_or(DELETED_FLAG.clone()))
        .and_where(Messages::Id.equals(Var::of(Messages::Id)))
        .and_where(
            // prevent double-updates
            Messages::Flags.bit_and(DELETED_FLAG.clone()).equals(0i16.lit()),
        )
}

fn delete_if_own() -> impl AnyQuery {
    use schema::*;

    Query::update()
        .table::<Messages>()
        .set(Messages::Flags, Messages::Flags.bit_or(DELETED_FLAG.clone()))
        .and_where(Messages::Id.equals(Var::of(Messages::Id)))
        .and_where(Messages::UserId.equals(Var::of(Users::Id)))
        .and_where(
            // prevent double-updates
            Messages::Flags.bit_and(DELETED_FLAG.clone()).equals(0i16.lit()),
        )
}

fn delete_with_perms() -> impl AnyQuery {
    use schema::*;

    tables! {
        struct AggPerm {
            Perms: AggRoomPerms::Perms,
        }
    }

    const MANAGE_MESSAGE: i64 = Permission {
        party: PartyPermissions::empty(),
        room: RoomPermissions::MANAGE_MESSAGES,
        stream: StreamPermissions::empty(),
    }
    .pack() as i64;

    let msg_id_var = Var::at(Messages::Id, 1);
    let user_id_var = Var::at(Users::Id, 2);
    let room_id_var = Var::at(Rooms::Id, 3);

    let permissions = AggPerm::as_query(
        Query::select()
            .expr(AggRoomPerms::Perms.alias_to(AggPerm::Perms))
            .from_table::<AggRoomPerms>()
            .and_where(AggRoomPerms::UserId.equals(user_id_var.clone()))
            .and_where(AggRoomPerms::RoomId.equals(room_id_var.clone())),
    );

    Query::with()
        .with(permissions)
        .update()
        .table::<Messages>()
        .and_where(
            AggPerm::Perms
                .bit_and(MANAGE_MESSAGE.lit())
                .equals(MANAGE_MESSAGE.lit())
                .or(Messages::UserId.equals(user_id_var)),
        )
        .set(Messages::Flags, Messages::Flags.bit_or(DELETED_FLAG.clone()))
        .and_where(Messages::Id.equals(msg_id_var))
        .and_where(
            // prevent double-updates
            Messages::Flags.bit_and(DELETED_FLAG.clone()).equals(0i16.lit()),
        )
}
