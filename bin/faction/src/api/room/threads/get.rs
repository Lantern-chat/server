use schema::Snowflake;

use sdk::models::*;

use crate::prelude::*;
pub async fn get_thread(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
    thread_id: Snowflake,
) -> Result<Thread, Error> {
    unimplemented!()

    // let had_perms = if let Some(perm) = state.perm_cache.get(auth.user_id, room_id).await {
    //     if !perm.contains(RoomPermissions::READ_MESSAGE_HISTORY) {
    //         return Err(Error::NotFound);
    //     }

    //     true
    // } else {
    //     false
    // };

    // let db = state.db.read.get().await?;

    // let row = if had_perms {
    //     db.query_opt_cached_typed(|| get_thread_without_perms(), &[&thread_id])
    //         .await?
    // } else {
    //     db.query_opt_cached_typed(|| get_thread_with_perms(), &[&auth.user_id, &room_id, &thread_id])
    //         .await?
    // };

    // match row {
    //     None => Err(Error::NotFound),
    //     Some(row) => {
    //         let msg = match get_one::parse_msg(&state, &row) {
    //             Ok(msg) => msg,
    //             Err(Error::NotFound) => unimplemented!(),
    //             Err(e) => return Err(e),
    //         };

    //         Ok(Thread {
    //             id: thread_id,
    //             parent: msg,
    //             flags: row.try_get(ThreadColumns::Flags as usize)?,
    //         })
    //     }
    // }
}

// use schema::Threads;

// thorn::indexed_columns! {
//     pub enum ThreadColumns continue get_one::Columns {
//         Threads::Flags,
//     }
// }

// fn get_thread_without_perms() -> impl thorn::AnyQuery {
//     use schema::*;
//     use thorn::*;

//     Query::select()
//         .from(Threads::inner_join_table::<AggMessages>().on(AggMessages::MsgId.equals(Threads::ParentId)))
//         .and_where(Threads::Id.equals(Var::of(Threads::Id)))
//         .cols(get_one::Columns::default())
//         .cols(ThreadColumns::default())
// }

// fn get_thread_with_perms() -> impl thorn::AnyQuery {
//     use schema::*;
//     use thorn::*;

//     tables! {
//         struct AggPerm {
//             Perms: AggRoomPerms::Perms,
//         }
//     }

//     const READ_MESSAGES: i64 = Permission::PACKED_READ_MESSAGE_HISTORY as i64;

//     let user_id_var = Var::at(Users::Id, 1);
//     let room_id_var = Var::at(Rooms::Id, 2);
//     let thread_id_var = Var::at(Threads::Id, 3);

//     let permissions = AggPerm::as_query(
//         Query::select()
//             .expr(AggRoomPerms::Perms.alias_to(AggPerm::Perms))
//             .from_table::<AggRoomPerms>()
//             .and_where(AggRoomPerms::UserId.equals(user_id_var.clone()))
//             .and_where(AggRoomPerms::RoomId.equals(room_id_var.clone())),
//     );

//     Query::with()
//         .with(permissions)
//         .select()
//         .from(Threads::inner_join_table::<AggMessages>().on(AggMessages::MsgId.equals(Threads::ParentId)))
//         .and_where(Threads::Id.equals(thread_id_var))
//         .and_where(
//             AggPerm::Perms
//                 .bitand(READ_MESSAGES.lit())
//                 .equals(READ_MESSAGES.lit()),
//         )
//         .cols(get_one::Columns::default())
//         .cols(ThreadColumns::default())
// }
