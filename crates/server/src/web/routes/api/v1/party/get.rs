use either::Either;
use ftl::*;

use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, party::get::get_party},
    web::routes::api::ApiError,
    ServerState,
};

pub async fn get(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> impl Reply {
    match get_party(route.state, auth, party_id).await {
        Ok(ref party) => Either::Left(reply::json(party)),
        Err(e) => Either::Right(ApiError::err(e)),
    }
}
