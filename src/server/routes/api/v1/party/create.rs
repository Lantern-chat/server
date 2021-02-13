use std::{sync::Arc, time::SystemTime};

use warp::{
    body::json,
    hyper::{Server, StatusCode},
    reject::Reject,
    Filter, Rejection, Reply,
};

use crate::{
    db::{ClientError, Snowflake},
    server::{
        auth::AuthToken,
        rate::RateLimitKey,
        routes::{
            api::ApiError,
            filters::{auth, Authorization},
        },
        ServerState,
    },
};

#[derive(Debug, Clone, Deserialize)]
struct PartyCreateForm {
    name: String,
}

pub fn create(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::post()
        .and(warp::path::end())
        .and(auth(state.clone()))
        .and(warp::body::form::<PartyCreateForm>())
        .map(move |auth, form| (auth, form, state.clone()))
        .map(|(auth, form, state)| /*TODO*/ warp::reply::reply())
}

pub struct CreatePartyError {}

async fn create_party(auth: Authorization) -> Result<(), CreatePartyError> {
    return Ok(());
}
