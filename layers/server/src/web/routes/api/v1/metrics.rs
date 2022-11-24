use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwapOption;
use bytes::Bytes;
use headers::ContentType;

use ftl::*;

use crate::backend::api::metrics::MetricsOptions;
use crate::{Error, ServerState};

#[async_recursion]
pub async fn metrics(route: Route<ServerState>) -> Result<Response, Error> {
    let options = match route.query::<MetricsOptions>() {
        Some(res) => res?,
        None => MetricsOptions::default(),
    };

    let stream = crate::backend::api::metrics::get_metrics(&route.state, options).await?;

    Ok(reply::json::map_stream(stream).into_response())
}
