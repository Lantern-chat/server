use futures::StreamExt;
use thorn::pg::Json;

use crate::backend::{
    api::room::messages::get::get_one,
    util::encrypted_asset::{encrypt_snowflake, encrypt_snowflake_opt},
};

use super::prelude::*;

pub async fn message_create(state: &ServerState, db: &db::pool::Client, id: Snowflake) -> Result<(), Error> {
    let msg = get_one(state.clone(), db, id).await?;

    if let Some(party_id) = msg.party_id {
        let room_id = msg.room_id;

        let event = ServerMsg::new_message_create(msg);

        state.gateway.broadcast_event(Event::new(event, Some(room_id))?, party_id);
    }

    Ok(())
}
