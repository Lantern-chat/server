use super::prelude::*;

pub async fn message_delete(
    state: &ServerState,
    db: &db::pool::Client,
    id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    #[rustfmt::skip]
    let Some(row) = db.query_opt2(schema::sql! {
        SELECT Messages.RoomId AS @RoomId, Messages.UserId AS @UserId
        FROM Messages WHERE Messages.Id = #{&id as Messages::Id}
    }?).await? else { return Ok(()); };

    let room_id = row.room_id()?;
    let user_id = row.user_id()?;

    let event = ServerMsg::new_message_delete(MessageDeleteEvent {
        id,
        room_id,
        user_id,
        party_id,
    });

    if let Some(party_id) = party_id {
        state.gateway.broadcast_event(Event::new(event, Some(room_id))?, party_id).await;
    }

    Ok(())
}
