use ftl::*;
use sdk::Snowflake;

use crate::{web::response::WebResult, Error, ServerState};

pub mod asset;
pub mod attachments;

#[rustfmt::skip]
pub async fn cdn(route: Route<ServerState>) -> Response {
    let encoding = match route.query::<crate::web::encoding::EncodingQuery>() {
        Some(Ok(q)) => q.encoding,
        _ => sdk::driver::Encoding::JSON,
    };

    crate::web::response::web_response(encoding, real_cdn(route).await)
}

async fn real_cdn(mut route: Route<ServerState>) -> WebResult {
    let config = route.state.config();
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
