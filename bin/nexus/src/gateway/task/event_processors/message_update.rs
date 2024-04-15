use super::prelude::*;

pub async fn message_update(state: &ServerState, db: &db::pool::Client, id: MessageId) -> Result<(), Error> {
    let msg = crate::api::room::messages::get::get_one(state.clone(), db, id).await?;

    #[rustfmt::skip]
    state.gateway.events.send_simple(&ServerEvent::party(
        msg.party_id,
        Some(msg.room_id),
        ServerMsg::new_message_update(msg),
    )).await;

    Ok(())
}
