use super::prelude::*;

pub async fn message_update(state: &ServerState, db: &db::Client, id: MessageId) -> Result<(), Error> {
    let msg = crate::internal::get_messages::get_one(state.clone(), db, id).await?;

    #[rustfmt::skip]
    state.gateway.events.send(&ServerEvent::party(
        msg.party_id,
        Some(msg.room_id),
        ServerMsg::new_message_update(msg),
    )).await?;

    Ok(())
}
