use ftl::*;

use crate::ServerState;

use self::user_avatar::user_avatar;

pub mod attachments;
pub mod user_avatar;

pub async fn cdn(mut route: Route<ServerState>) -> Response {
    match route.host() {
        Some(host) if host.as_str().starts_with("cdn") => {}
        _ => return StatusCode::NOT_FOUND.into_response(),
    }

    match route.next().segment() {
        Exact("attachments") => attachments::attachments(route).await,
        Exact("avatar") => user_avatar(route).await,
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
