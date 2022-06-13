use thorn::pg::Json;

use crate::{ctrl::util::encrypted_asset::encrypt_snowflake, web::gateway::Event};

use sdk::models::gateway::message::ServerMsg;

use super::prelude::*;

pub async fn message_update(
    state: &ServerState,
    db: &db::pool::Client,
    id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let msg = super::message_create::get_message(state, db, id, party_id).await?;

    if let Some(party_id) = msg.party_id {
        let room_id = msg.room_id;

        let event = ServerMsg::new_message_update(msg);

        state
            .gateway
            .broadcast_event(Event::new(event, Some(room_id))?, party_id)
            .await;
    }

    Ok(())
}
