use either::Either;
use ftl::*;

use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, party::get::get_party},
    web::routes::api::ApiError,
    ServerState,
};

pub async fn get(route: Route<ServerState>, auth: Authorization, party_id: Snowflake) -> Response {
    match get_party(route.state, auth, party_id).await {
        Ok(ref party) => reply::json(party).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
