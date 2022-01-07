use thorn::pg::Json;

use crate::{
    ctrl::util::encrypted_asset::encrypt_snowflake,
    web::gateway::{msg::ServerMsg, Event},
};

use super::*;

pub async fn message_update(
    state: &ServerState,
    db: &db::pool::Client,
    id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let msg = super::message_create::get_message(state, db, id, party_id).await?;

    if let Some(party_id) = msg.party_id {
        let room_id = msg.room_id;

        let event = ServerMsg::new_messageupdate(msg);

        state
            .gateway
            .broadcast_event(Event::new(event, Some(room_id))?, party_id)
            .await;
    }

    Ok(())
}