use ftl::*;

use db::Snowflake;

pub async fn options(route: Route<crate::ServerState>) -> impl Reply {
    let mut res = Response::default();

    *res.status_mut() = StatusCode::NO_CONTENT;

    res.headers_mut().extend(
        super::TUS_HEADERS
            .iter()
            .map(|(k, v)| (k.clone(), v.clone())),
    );

    res
}
