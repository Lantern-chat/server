use crate::web::gateway::{msg::ServerMsg, Event};

use super::*;

pub async fn presence_updated(
    state: &ServerState,
    db: &db::pool::Client,
    id: Snowflake,
) -> Result<(), Error> {
    unimplemented!()
}
