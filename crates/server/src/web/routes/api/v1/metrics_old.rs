use std::sync::Arc;
use std::time::{Duration, Instant};

use arc_swap::ArcSwapOption;
use bytes::Bytes;
use headers::ContentType;

use ftl::*;

use crate::{
    metric::{MemoryMetrics, Metrics, API_METRICS, MEMORY_METRICS},
    web::encoding::{bytes_as_json, bytes_as_msgpack, Encoding, EncodingQuery},
    ServerState,
};

#[derive(Serialize)]
struct AllMetrics {
    percentiles: [u16; 3],
    memory: &'static MemoryMetrics,
    api: &'static Metrics,
}

struct MetricsCache {
    ts: Instant,
    json: Bytes,
    msgpack: Bytes,
}

impl MetricsCache {
    pub fn at(ts: Instant) -> Self {
        let metrics = AllMetrics {
            percentiles: API_METRICS.percentiles(),
            memory: &MEMORY_METRICS,
            api: &API_METRICS,
        };

        Self {
            ts,
            json: serde_json::to_vec(&metrics).unwrap().into(),
            msgpack: rmp_serde::to_vec(&metrics).unwrap().into(),
        }
    }
}

static CACHE: ArcSwapOption<MetricsCache> = ArcSwapOption::const_empty();

pub fn metrics(route: Route<ServerState>) -> Response {
    #[cfg(debug_assertions)]
    const REFRESH_DURATION: Duration = Duration::from_secs(5);

    #[cfg(not(debug_assertions))]
    const REFRESH_DURATION: Duration = Duration::from_secs(60);

    let cache = CACHE.load();
    match &*cache {
        Some(cache) if route.start.duration_since(cache.ts) < REFRESH_DURATION => {
            match route.query::<EncodingQuery>() {
                Some(Ok(EncodingQuery {
                    encoding: Encoding::MsgPack,
                })) => bytes_as_msgpack(cache.msgpack.clone()),
                _ => bytes_as_json(cache.json.clone()),
            }
        }
        _ => {
            CACHE.store(Some(Arc::new(MetricsCache::at(route.start))));
            return metrics(route);
        }
    }
}
