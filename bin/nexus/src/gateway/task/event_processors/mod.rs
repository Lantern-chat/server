use db::pool::Client;

use crate::prelude::*;

pub mod prelude {
    pub use crate::prelude::*;

    pub use sdk::models::{
        gateway::{events::*, message::ServerMsg, Intent},
        *,
    };
}

pub mod member_event;
pub mod message_create;
pub mod message_delete;
pub mod message_update;
pub mod presence_update;
pub mod profile_event;
pub mod role_event;
pub mod user_event;

#[derive(Debug, Clone, Copy)]
pub struct RawEvent {
    pub id: Snowflake,
    pub room_id: Option<RoomId>,
    pub code: EventCode,
}

use schema::EventCode;

#[allow(unused_variables)]
pub async fn process(
    state: &ServerState,
    db: &Client,
    event: RawEvent,
    party_id: Option<PartyId>,
) -> Result<(), Error> {
    let RawEvent { id, code, room_id } = event;

    match code {
        EventCode::MessageCreate => message_create::message_create(state, db, id).await,
        EventCode::MessageDelete => message_delete::message_delete(state, db, id, party_id, room_id).await,
        EventCode::MessageUpdate => message_update::message_update(state, db, id).await,
        EventCode::PresenceUpdated => presence_update::presence_updated(state, db, id, party_id).await,
        EventCode::MemberJoined
        | EventCode::MemberLeft
        | EventCode::MemberUpdated
        | EventCode::MemberBan
        | EventCode::MemberUnban => member_event::member_event(state, code, db, id, party_id).await,
        EventCode::RoleCreated | EventCode::RoleUpdated | EventCode::RoleDeleted => {
            role_event::role_event(state, code, db, id, party_id).await
        }
        EventCode::SelfUpdated => user_event::self_update(state, db, id, party_id).await,
        EventCode::UserUpdated => user_event::user_update(state, db, id).await,
        EventCode::ProfileUpdated => profile_event::profile_updated(state, db, id, party_id).await,
        _ => Err(Error::Unimplemented),
    }
}
