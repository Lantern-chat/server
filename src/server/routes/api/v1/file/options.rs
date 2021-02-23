use http::StatusCode;

use crate::{db::Snowflake, server::ftl::*};

pub async fn options(mut route: Route) -> impl Reply {
    let mut res = Response::default();

    *res.status_mut() = StatusCode::NO_CONTENT;

    res.headers_mut().extend(
        super::TUS_HEADERS
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );

    res
}
