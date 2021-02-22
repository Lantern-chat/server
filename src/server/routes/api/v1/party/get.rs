use rand::Rng;
use std::{sync::Arc, time::SystemTime};

use http::StatusCode;

use crate::{
    db::{ClientError, Snowflake},
    server::{
        routes::api::{auth::Authorization, util::time::is_of_age},
        ServerState,
    },
};

use crate::server::ftl::*;

pub async fn get(route: Route, auth: Authorization, party_id: Snowflake) -> impl Reply {
    "There will be a party here"
}
