use rand::Rng;
use std::{sync::Arc, time::SystemTime};

use ftl::*;

use db::{schema::Party, ClientError, Snowflake};

use crate::{
    routes::api::{auth::Authorization, util::time::is_of_age},
    ServerState,
};

pub async fn get(
    route: Route<ServerState>,
    auth: Authorization,
    party_id: Snowflake,
) -> impl Reply {
    match Party::find(&route.state.db, party_id).await {
        Ok(Some(ref party)) => Ok(reply::json(party)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(e) => {
            log::error!("GetParty Error: {}", e);

            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
