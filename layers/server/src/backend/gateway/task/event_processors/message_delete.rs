use crate::backend::gateway::Event;

use sdk::models::gateway::{events::MessageDeleteEvent, message::ServerMsg};

use super::prelude::*;

pub async fn message_delete(
    state: &ServerState,
    db: &db::pool::Client,
    id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let row = db
        .query_opt_cached_typed(
            || {
                use schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<Messages>()
                    .col(Messages::RoomId)
                    .and_where(Messages::Id.equals(Var::of(Messages::Id)))
            },
            &[&id],
        )
        .await?;

    let room_id = match row {
        Some(row) => row.try_get(0)?,
        None => return Ok(()),
    };

    let event = ServerMsg::new_message_delete(MessageDeleteEvent {
        id,
        room_id,
        party_id,
    });

    if let Some(party_id) = party_id {
        state
            .gateway
            .broadcast_event(Event::new(event, Some(room_id))?, party_id)
            .await;
    }

    Ok(())
}
