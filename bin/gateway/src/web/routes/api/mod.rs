pub mod v1;

use ftl::*;

// import all these to be used by child modules
use crate::web::auth::MaybeAuth;
use crate::web::response::{RouteResult, WebResponse, WebResult};
use crate::{Authorization, Error, ServerState};
use sdk::Snowflake;

#[rustfmt::skip]
pub async fn api(mut route: Route<ServerState>) -> WebResult {
    match route.next().segment() {
        // ANY /api/v1
        Exact("v1") => v1::api_v1(route).await,
        _ => Err(Error::NotFound),
    }
}
