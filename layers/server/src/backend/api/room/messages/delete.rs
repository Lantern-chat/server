use schema::{Snowflake, SnowflakeExt};
use smol_str::SmolStr;

use crate::{Authorization, Error, ServerState};

use sdk::models::*;

pub async fn delete_msg(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> Result<(), Error> {
    let perm = state.perm_cache.get(auth.user_id, room_id).await;

    if let Some(ref perms) = perm {
        // user cannot view channel at all
        if !perms.contains(Permissions::READ_MESSAGE_HISTORY) {
            return Err(Error::Unauthorized);
        }
    }

    let db = state.db.write.get().await?;

    use arrayvec::ArrayVec;

    let mut params: ArrayVec<&(dyn pg::ToSql + Sync), 3> =
        ArrayVec::from([&msg_id as _, &auth.user_id as _, &room_id as _]);

    let query = match perm {
        Some(perm) => {
            if perm.contains(Permissions::MANAGE_MESSAGES) {
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

const DELETED_FLAG: i16 = MessageFlags::DELETED.bits();

///!!! NOTE: All parameters of these are sorted as: [msg_id, user_id, room_id]

fn delete_without_perms() -> impl AnyQuery {
    use schema::*;

    Query::update()
        .table::<Messages>()
        .set(Messages::Flags, Messages::Flags.bitor(DELETED_FLAG.lit()))
        .and_where(Messages::Id.equals(Var::of(Messages::Id)))
        .and_where(Messages::Flags.has_no_bits(DELETED_FLAG.lit())) // prevent double-updates
}

fn delete_if_own() -> impl AnyQuery {
    use schema::*;

    Query::update()
        .table::<Messages>()
        .set(Messages::Flags, Messages::Flags.bitor(DELETED_FLAG.lit()))
        .and_where(Messages::Id.equals(Var::of(Messages::Id)))
        .and_where(Messages::UserId.equals(Var::of(Users::Id)))
        .and_where(Messages::Flags.has_no_bits(DELETED_FLAG.lit())) // prevent double-updates
}

fn delete_with_perms() -> impl AnyQuery {
    use schema::*;

    tables! {
        struct AggPerm {
            Permissions1: AggRoomPerms::Permissions1,
            Permissions2: AggRoomPerms::Permissions2,
        }
    }

    let msg_id_var = Var::at(Messages::Id, 1);
    let user_id_var = Var::at(Users::Id, 2);
    let room_id_var = Var::at(Rooms::Id, 3);

    let permissions = AggPerm::as_query(
        Query::select()
            .expr(AggRoomPerms::Permissions1.alias_to(AggPerm::Permissions1))
            .expr(AggRoomPerms::Permissions2.alias_to(AggPerm::Permissions2))
            .from_table::<AggRoomPerms>()
            .and_where(AggRoomPerms::UserId.equals(user_id_var.clone()))
            .and_where(AggRoomPerms::RoomId.equals(room_id_var)),
    );

    Query::with()
        .with(permissions)
        .update()
        .table::<Messages>()
        .and_where(
            schema::has_all_permission_bits(
                Permissions::MANAGE_MESSAGES,
                (AggPerm::Permissions1, AggPerm::Permissions2),
            )
            .or(Messages::UserId.equals(user_id_var)),
        )
        .set(Messages::Flags, Messages::Flags.bitor(DELETED_FLAG.lit()))
        .and_where(Messages::Id.equals(msg_id_var))
        .and_where(Messages::Flags.has_no_bits(DELETED_FLAG.lit())) // prevent double-updates
}
