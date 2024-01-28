use rand::Rng;
use std::{sync::Arc, time::SystemTime};

use db::{schema::Party, ClientError, Snowflake};

use crate::{
    routes::api::{auth::Authorization, util::time::is_of_age},
    ServerState,
};

use ftl::*;

pub async fn delete(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> Response {
    "Unimplemented".into_response()
}
