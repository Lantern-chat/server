use super::prelude::*;

pub async fn message_create(state: &ServerState, db: &db::pool::Client, id: Snowflake) -> Result<(), Error> {
    let msg = crate::api::room::messages::get::get_one(state.clone(), db, id).await?;

    if let Some(party_id) = msg.party_id {
        let room_id = msg.room_id;

        #[rustfmt::skip]
        state.gateway.events.send_simple(&ServerEvent::party(
            ServerMsg::new_message_create(msg),
            party_id,
            Some(room_id),
        )).await;
    }

    Ok(())
}
