pub mod error;

pub mod v1;

use ftl::*;

use crate::ServerState;

pub async fn api(mut route: Route<ServerState>) -> Response {
    match route.next().segment() {
        // ANY /api/v1
        Exact("v1") => v1::api_v1(route).await.into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
