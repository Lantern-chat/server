use futures::{StreamExt, TryStreamExt};

use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, Error, SearchMode},
    ServerState,
};

use models::*;

pub async fn get_room(
    state: ServerState,
    auth: Authorization,
    room_id: Snowflake,
) -> Result<Room, Error> {
    unimplemented!()
}
