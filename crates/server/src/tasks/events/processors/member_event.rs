use std::sync::Arc;

use schema::EventCode;

use crate::web::gateway::{
    msg::{server::UserPresenceInner, ServerMsg},
    Event,
};

use super::*;

pub async fn member_event(
    state: &ServerState,
    event: EventCode,
    db: &db::pool::Client,
    id: Snowflake,
    party_id: Option<Snowflake>,
) -> Result<(), Error> {
    let party_id = match party_id {
        Some(party_id) => party_id,
        None => {
            return Err(Error::InternalError(format!(
                "Member Event without a party id!: {:?} - {}",
                event, id
            )));
        }
    };

    Ok(())
}
