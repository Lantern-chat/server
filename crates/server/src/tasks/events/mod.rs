use schema::Snowflake;

use crate::{ctrl::Error, ServerState};

pub mod cache;
pub mod task;

pub mod processors {
    use thorn::*;

    use models::*;
    use schema::Snowflake;

    use crate::{ctrl::Error, ServerState};

    pub mod message_create;
    pub mod typing_start;
}

#[derive(Debug, Clone, Copy)]
pub struct RawEvent {
    pub id: Snowflake,
    pub room_id: Option<Snowflake>,
    pub code: EventCode,
}

use schema::codes::EventCode;

pub async fn process(state: &ServerState, event: RawEvent, party_id: Option<Snowflake>) -> Result<(), Error> {
    let party_id_res = party_id.ok_or_else(|| Error::InternalErrorStatic("Missing PartyId"));

    match event.code {
        EventCode::MessageCreate => {
            processors::message_create::message_create(state, event.id, party_id).await?;
        }
        EventCode::TypingStarted => {
            if let Some(room_id) = event.room_id {
                processors::typing_start::trigger_typing(state, event.id, party_id, room_id).await?;
            } else {
                log::warn!("Typing started outside of room!");
            }
        }
        _ => unimplemented!(),
    }

    Ok(())
}
