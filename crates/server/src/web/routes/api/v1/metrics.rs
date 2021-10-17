use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwapOption;
use bytes::Bytes;
use headers::ContentType;

use ftl::*;

use crate::ctrl::metrics::MetricsOptions;
use crate::web::routes::api::ApiError;
use crate::ServerState;

pub async fn metrics(route: Route<ServerState>) -> Response {
    let options = match route.query::<MetricsOptions>() {
        None => MetricsOptions::default(),
        Some(Ok(options)) => options,
        Some(Err(e)) => return ApiError::err(e.into()).into_response(),
    };

    match crate::ctrl::metrics::get_metrics(route.state, options).await {
        Ok(stream) => reply::json_map_stream(stream).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
