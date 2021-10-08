use schema::{Snowflake, SnowflakeExt};
use smol_str::SmolStr;

use crate::{
    ctrl::{auth::Authorization, perm::get_cached_room_permissions, Error, SearchMode},
    ServerState,
};

use models::*;

pub async fn delete_msg(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    msg_id: Snowflake,
) -> Result<(), Error> {
    let has_perms = match state.perm_cache.get(auth.user_id, room_id).await {
        Some(perm) if !perm.perm.room.contains(RoomPermissions::MANAGE_MESSAGES) => {
            return Err(Error::Unauthorized);
        }
        Some(_) => true,
        None => false,
    };

    let db = state.db.write.get().await?;

    let rows_updated = match has_perms {
        true => {
            db.execute_cached_typed(|| delete_without_perms(), &[&msg_id])
                .await
        }
        false => {
            db.execute_cached_typed(|| delete_with_perms(), &[&auth.user_id, &room_id, &msg_id])
                .await
        }
    };

    match rows_updated {
        Ok(1) => Ok(()),
        Ok(_) => Err(Error::Unauthorized),
        Err(e) => Err(e.into()),
    }
}

use thorn::*;

fn delete_without_perms() -> impl AnyQuery {
    use schema::*;

    let deleted_flag = Literal::Int2(MessageFlags::DELETED.bits());

    Query::update()
        .table::<Messages>()
        .set(Messages::Flags, Messages::Flags.bit_or(deleted_flag.clone()))
        .and_where(Messages::Id.equals(Var::of(Messages::Id)))
        .and_where(
            // prevent double-updates
            Messages::Flags
                .bit_and(deleted_flag.clone())
                .equals(Literal::Int2(0)),
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

    let user_id_var = Var::at(Users::Id, 1);
    let room_id_var = Var::at(Rooms::Id, 2);
    let msg_id_var = Var::at(Messages::Id, 3);

    let deleted_flag = Literal::Int2(MessageFlags::DELETED.bits());

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
                .bit_and(Literal::Int8(MANAGE_MESSAGE))
                .equals(Literal::Int8(MANAGE_MESSAGE)),
        )
        .set(Messages::Flags, Messages::Flags.bit_or(deleted_flag.clone()))
        .and_where(Messages::Id.equals(msg_id_var))
        .and_where(
            // prevent double-updates
            Messages::Flags
                .bit_and(deleted_flag.clone())
                .equals(Literal::Int2(0)),
        )
}
