pub mod v1;

use ftl::*;

use crate::{Error, ServerState};

pub async fn api(mut route: Route<ServerState>) -> Result<Response, Error> {
    match route.next().segment() {
        // ANY /api/v1
        Exact("v1") => v1::api_v1(route).await,
        _ => Err(Error::NotFound),
    }
}
