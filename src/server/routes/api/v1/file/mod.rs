use http::{Method, StatusCode};

use crate::{db::Snowflake, server::ftl::*};

pub async fn file(mut route: Route) -> impl Reply {
    match route.next().method_segment() {
        // POST /api/v1/file
        (&Method::POST, End) => "Upload".into_response(),

        // ANY /api/v1/file/1234
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(file_id)) => "".into_response(),
            _ => StatusCode::BAD_REQUEST.into_response(),
        },

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
