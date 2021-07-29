use ftl::*;

use crate::ServerState;

pub mod attachments;

pub async fn cdn(mut route: Route<ServerState>) -> Response {
    match route.host() {
        Some(host) if host.as_str().starts_with("cdn") => {}
        _ => return StatusCode::NOT_FOUND.into_response(),
    }

    match route.next().segment() {
        Exact("attachments") => attachments::attachments(route).await,
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
