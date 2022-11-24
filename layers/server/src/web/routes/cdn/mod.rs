use ftl::*;
use sdk::Snowflake;

use crate::{Error, ServerState};

pub mod asset;
pub mod attachments;

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
        Exact("attachments") => attachments::attachments(route).await,
        Exact("user" | "room" | "party" | "role" | "emote" | "sticker") => asset::asset(route).await,
        _ => Err(Error::NotFound),
    }
}
