pub mod v1;

use ftl::{body, End, Exact, Method, Reply, Response, Route, StatusCode};

// import all these to be used by child modules
use crate::prelude::*;
use crate::web::auth::MaybeAuth;
use sdk::Snowflake;

use async_recursion::async_recursion;

#[rustfmt::skip]
pub async fn api(mut route: Route<ServerState>) -> Response {
    match route.next().segment() {
        // ANY /api/v1
        Exact("v1") => v1::api_v1(route).await,
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
