use db::Snowflake;

use crate::{ctrl::Error, ServerState};

pub mod task;

pub mod processors {
    use thorn::*;

    use db::Snowflake;

    use crate::{ctrl::Error, ServerState};

    pub mod message_create;
}

#[derive(Debug, Clone, Copy)]
pub struct RawEventCode {
    pub id: Snowflake,
    pub code: i16,
}

use db::schema::codes::EventCode;

pub async fn process(
    state: &ServerState,
    event: RawEventCode,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let code = match EventCode::from_i16(event.code) {
        Some(code) => code,
        None => {
            return Err(Error::InternalError(format!(
                "Unknown event code: {}",
                event.code
            )));
        }
    };

    let party_id_res = party_id.ok_or_else(|| Error::InternalErrorStatic("Missing PartyId"));

    match code {
        EventCode::MessageCreate => {
            processors::message_create::message_create(state, event.id, party_id).await?
        }
        _ => unimplemented!(),
    }

    Ok(())
}