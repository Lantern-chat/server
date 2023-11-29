use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwapOption;
use bytes::Bytes;
use headers::ContentType;

use super::*;

use crate::backend::api::metrics::MetricsOptions;

#[async_recursion]
pub async fn metrics(route: Route<ServerState>) -> WebResult {
    let options = match route.query::<MetricsOptions>() {
        Some(res) => res?,
        None => MetricsOptions::default(),
    };

    Ok(WebResponse::stream(
        crate::backend::api::metrics::get_metrics(&route.state, options).await?,
    ))
}
