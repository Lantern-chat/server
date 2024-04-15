use super::prelude::*;

pub async fn message_delete(
    state: &ServerState,
    _db: &db::pool::Client,
    id: MessageId,
    party_id: Option<PartyId>,
    room_id: Option<RoomId>,
) -> Result<(), Error> {
    let (Some(party_id), Some(room_id)) = (party_id, room_id) else {
        return Ok(());
    };

    state
        .gateway
        .events
        .send_simple(&ServerEvent::party(
            party_id,
            Some(room_id),
            ServerMsg::new_message_delete(MessageDeleteEvent { id, room_id, party_id }),
        ))
        .await;

    Ok(())
}
