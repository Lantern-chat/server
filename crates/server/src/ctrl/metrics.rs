use std::sync::Arc;
use std::time::SystemTime;

#[derive(Clone, Copy)]
pub struct Metrics {
    pub ts: Option<SystemTime>,
    pub mem: i64,
    pub upload: i64,
    pub msgs: i32,
    pub reqs: i32,
    pub errs: i32,
    pub conns: i32,
    pub p50: i16,
    pub p95: i16,
    pub p99: i16,
}

use crate::{
    metric::{ApiMetrics, API_METRICS, MEMORY_METRICS},
    ServerState,
};

impl Metrics {
    pub fn acquire(state: &ServerState) -> Self {
        let metrics = API_METRICS.swap(Arc::new(ApiMetrics::default()));

        let (count, [p50, p95, p99]) = metrics.percentiles();

        Metrics {
            ts: None,
            mem: MEMORY_METRICS.allocated.get() as i64,
            upload: 0,
            msgs: 0,
            reqs: count as i32,
            errs: metrics.errs.get() as i32,
            conns: state.gateway.conns.len() as i32,
            p50: p50 as i16,
            p95: p95 as i16,
            p99: p99 as i16,
        }
    }
}

use futures::{Stream, StreamExt};

use crate::ctrl::Error;

// pub async fn read_metrics(state: ServerState) -> Result<impl Stream<Item = Result<Metrics, Error>>, Error> {
//     let db = state.db.read.get().await?;

//     let stream = db
//         .query_stream_cached_typed(
//             || {
//                 use schema::*;
//                 use thorn::*;

//                 Query::select().from_table::<Metrics>()
//             },
//             &[],
//         )
//         .await?;
// }
