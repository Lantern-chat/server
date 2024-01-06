use super::prelude::*;

pub async fn message_delete(
    state: &ServerState,
    _db: &db::pool::Client,
    id: Snowflake,
    party_id: Option<Snowflake>,
    room_id: Option<Snowflake>,
) -> Result<(), Error> {
    let Some(room_id) = room_id else {
        return Ok(());
    };

    let event = ServerMsg::new_message_delete(MessageDeleteEvent { id, room_id, party_id });

    if let Some(party_id) = party_id {
        state.gateway.events.send_simple(&ServerEvent::party(event, party_id, Some(room_id))).await;
    } else {
        log::error!("Unimplemented: message_delete for non-party");
    }

    Ok(())
}
