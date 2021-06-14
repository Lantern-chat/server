use db::{Snowflake, SnowflakeExt};

use crate::{
    ctrl::{auth::Authorization, perm::get_cached_room_permissions, Error, SearchMode},
    ServerState,
};

use models::*;

pub async fn trigger_typing(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
) -> Result<(), Error> {
    let permissions = get_cached_room_permissions(&state, auth.user_id, room_id).await?;

    if !permissions.room.contains(RoomPermissions::SEND_MESSAGES) {
        return Err(Error::NotFound);
    }

    let db = state.db.write.get().await?;

    db.execute_cached_typed(
        || {
            use db::schema::*;
            use thorn::*;

            tables! {
                struct AggPartyId {
                    PartyId: Party::Id,
                }
            }

            let user_id_var = Var::at(Users::Id, 1);
            let room_id_var = Var::at(Rooms::Id, 2);

            Query::with()
                .with(AggPartyId::as_query(
                    Query::select()
                        .expr(Party::Id.alias_to(AggPartyId::PartyId))
                        .from(Party::right_join_table::<Rooms>().on(Party::Id.equals(Rooms::PartyId)))
                        .and_where(Rooms::Id.equals(room_id_var.clone())),
                ))
                .insert()
                .into::<EventLog>()
                .cols(&[EventLog::Code, EventLog::Id, EventLog::PartyId, EventLog::RoomId])
                .query(
                    Query::select()
                        .from_table::<AggPartyId>()
                        .expr(Literal::Int2(EventCode::TypingStarted as i16))
                        .expr(user_id_var)
                        .expr(AggPartyId::PartyId)
                        .expr(room_id_var)
                        .as_value(),
                )
        },
        &[&auth.user_id, &room_id],
    )
    .await?;

    Ok(())
}
