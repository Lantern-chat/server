use super::prelude::*;

pub async fn message_update(state: &ServerState, db: &db::pool::Client, id: Snowflake) -> Result<(), Error> {
    let msg = crate::backend::api::room::messages::get2::get_one(state.clone(), db, id).await?;

    if let Some(party_id) = msg.party_id {
        let room_id = msg.room_id;

        let event = ServerMsg::new_message_update(msg);

        state.gateway.broadcast_event(Event::new(event, Some(room_id))?, party_id);
    }

    Ok(())
}
