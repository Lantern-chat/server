use db::pool::Client;
use schema::Snowflake;

use crate::{ctrl::Error, ServerState};

pub mod cache;
pub mod task;

pub mod processors {
    use thorn::*;

    use models::*;
    use schema::Snowflake;

    use crate::{ctrl::Error, ServerState};

    pub mod member_event;
    pub mod message_create;
    pub mod message_delete;
    pub mod presence_update;
    pub mod typing_start;
}

#[derive(Debug, Clone, Copy)]
pub struct RawEvent {
    pub id: Snowflake,
    pub room_id: Option<Snowflake>,
    pub code: EventCode,
}

use schema::codes::EventCode;

pub async fn process(
    state: &ServerState,
    db: &Client,
    event: RawEvent,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    //let party_id_res = party_id.ok_or_else(|| Error::InternalErrorStatic("Missing PartyId"));

    match event.code {
        EventCode::MessageCreate => {
            processors::message_create::message_create(state, db, event.id, party_id).await?;
        }
        EventCode::MessageDelete => {
            processors::message_delete::message_delete(state, db, event.id, party_id).await?;
        }
        EventCode::TypingStarted => {
            if let Some(room_id) = event.room_id {
                processors::typing_start::trigger_typing(state, db, event.id, party_id, room_id).await?;
            } else {
                log::warn!("Typing started outside of room!");
            }
        }
        EventCode::PresenceUpdated => {
            processors::presence_update::presence_updated(state, db, event.id).await?;
        }
        EventCode::MemberJoined
        | EventCode::MemberLeft
        | EventCode::MemberUpdated
        | EventCode::MemberBan
        | EventCode::MemberUnban => {
            processors::member_event::member_event(state, event.code, db, event.id, party_id).await?;
        }
        _ => unimplemented!(),
    }

    Ok(())
}
