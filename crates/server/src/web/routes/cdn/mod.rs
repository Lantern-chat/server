use ftl::*;

use crate::ServerState;

pub mod attachments;
pub mod avatar;

pub async fn cdn(mut route: Route<ServerState>) -> Response {
    match route.host() {
        Some(host) if host.as_str() == route.state.config.general.cdn_domain => {}
        _ => return StatusCode::NOT_FOUND.into_response(),
    }

    match route.next().segment() {
        Exact("attachments") => attachments::attachments(route).await,
        Exact("avatar") => {
            let kind = match route.next().segment() {
                Exact("user") => avatar::AvatarKind::User,
                Exact("room") => avatar::AvatarKind::Room,
                Exact("party") => avatar::AvatarKind::Party,
                Exact("role") => avatar::AvatarKind::Role,
                _ => return StatusCode::NOT_FOUND.into_response(),
            };

            avatar::avatar(route, kind).await
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
