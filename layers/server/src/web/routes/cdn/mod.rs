use ftl::*;
use futures::FutureExt;

use crate::{Error, ServerState};

pub mod attachments;
pub mod avatar;

use super::api::v1::ApiResponse;

pub async fn cdn(mut route: Route<ServerState>) -> ApiResponse {
    let config = &route.state.config;
    if config.web.strict_cdn {
        match route.host() {
            Some(host) if host.as_str() == config.web.cdn_domain => {}
            _ => return Err(Error::NotFound),
        }
    }

    match route.next().segment() {
        Exact("attachments") => attachments::attachments(route).boxed().await,
        Exact("avatar") => {
            let kind = match route.next().segment() {
                Exact("user") => avatar::AvatarKind::User,
                Exact("room") => avatar::AvatarKind::Room,
                Exact("party") => avatar::AvatarKind::Party,
                Exact("role") => avatar::AvatarKind::Role,
                _ => return Err(Error::NotFound),
            };

            avatar::avatar(route, kind).boxed().await
        }
        _ => Err(Error::NotFound),
    }
}
