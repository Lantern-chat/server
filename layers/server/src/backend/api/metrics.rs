use std::sync::Arc;
use std::time::SystemTime;

use parking_lot::Mutex;

/// A single set of metrics recorded for a single quanta
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Metrics {
    pub mem: i64,
    pub upload: i64,
    pub reqs: i32,
    pub errs: i32,
    pub conns: i32,
    pub events: i32,
    pub p50: i16,
    pub p95: i16,
    pub p99: i16,
}

impl Metrics {
    pub const fn default() -> Self {
        Metrics {
            mem: 0,
            upload: 0,
            reqs: 0,
            errs: 0,
            conns: 0,
            events: 0,
            p50: 0,
            p95: 0,
            p99: 0,
        }
    }

    /// Check fields that are summed up when aggregating for zeroes
    pub fn is_empty(&self) -> bool {
        self.upload == 0 && self.reqs == 0 && self.errs == 0 && self.events == 0
    }
}

/// Metrics aggregated over multiple quanta
#[derive(Clone, Copy, Serialize)]
pub struct AggregatedMetrics {
    pub mem: f32,
    pub upload: i64,
    pub reqs: i64,
    pub errs: i64,
    pub conns: f32,
    pub events: i64,
    pub p50: f32,
    pub p95: f32,
    pub p99: f32,
}

use crate::metrics::{ApiMetrics, API_METRICS, MEMORY_METRICS};

use crate::ServerState;

// This will always be uncontested due to the scheduling system.
static LAST_METRICS: Mutex<Metrics> = Mutex::new(Metrics::default());

impl Metrics {
    pub fn acquire(state: &ServerState) -> Option<Self> {
        let metrics = API_METRICS.swap(Arc::new(ApiMetrics::default()));

        let (count, [p50, p95, p99]) = metrics.percentiles();

        let new_metrics = Metrics {
            mem: MEMORY_METRICS.allocated.get() as i64,
            upload: metrics.upload.get() as i64,
            reqs: count as i32,
            errs: metrics.errs.get() as i32,
            conns: state.gateway.conns.len() as i32,
            events: metrics.events.get() as i32,
            p50: p50 as i16,
            p95: p95 as i16,
            p99: p99 as i16,
        };

        let mut last = LAST_METRICS.lock();
        if new_metrics.is_empty() && *last == new_metrics {
            return None;
        }
        *last = new_metrics;

        Some(new_metrics)
    }
}

use futures::{FutureExt, Stream, StreamExt};
use smol_str::SmolStr;
use timestamp::{formats::ShortMilliseconds, Duration, Timestamp, TimestampStr};

use crate::Error;

#[derive(Debug, Default, Clone, Deserialize)]
pub struct MetricsOptions {
    #[serde(default)]
    pub resolution: Option<u64>,

    #[serde(default)]
    pub start: Option<Timestamp>,
    #[serde(default)]
    pub end: Option<Timestamp>,
}

pub async fn get_metrics(
    state: &ServerState,
    options: MetricsOptions,
) -> Result<impl Stream<Item = Result<(TimestampStr<ShortMilliseconds>, AggregatedMetrics), Error>>, Error> {
    let MetricsOptions { resolution, start, end } = options;

    let seconds = 60
        * match resolution {
            Some(res) if res > 5 => res as i64,
            _ => 5,
        };

    #[rustfmt::skip]
    let stream = state.db.read.get().await?.query_stream2(schema::sql! {
        tables! {
            struct RoundedMetrics {
                RoundedTs: Type::INT8,
                Mem: Metrics::Mem,
                Upload: Metrics::Upload,
                Reqs: Metrics::Reqs,
                Errs: Metrics::Errs,
                Conns: Metrics::Conns,
                Events: Metrics::Events,
                P50: Metrics::P50,
                P95: Metrics::P95,
                P99: Metrics::P99,
            }
        };

        WITH RoundedMetrics AS (
            SELECT
                (ROUND(date_part("epoch", Metrics.Ts)) / #{&seconds as Type::INT8})::int8 * #{&seconds as Type::INT8}
                                AS RoundedMetrics.RoundedTs,
                Metrics.Mem     AS RoundedMetrics.Mem,
                Metrics.Upload  AS RoundedMetrics.Upload,
                Metrics.Reqs    AS RoundedMetrics.Reqs,
                Metrics.Errs    AS RoundedMetrics.Errs,
                Metrics.Conns   AS RoundedMetrics.Conns,
                Metrics.Events  AS RoundedMetrics.Events,
                Metrics.P50     AS RoundedMetrics.P50,
                Metrics.P95     AS RoundedMetrics.P95,
                Metrics.P99     AS RoundedMetrics.P99
            FROM Metrics
            WHERE TRUE
            if let Some(ref start) = start {
                AND Metrics.Ts >= #{start as Metrics::Ts}
            }
            if let Some(ref end) = end {
                AND Metrics.Ts < #{end as Metrics::Ts}
            }
            ORDER BY Metrics.Ts DESC
        )
        SELECT
            RoundedMetrics.RoundedTs            AS @RoundedTs,
            AVG(RoundedMetrics.Mem)::float4     AS @Mem,
            SUM(RoundedMetrics.Upload)::int8    AS @Upload,
            SUM(RoundedMetrics.Reqs)::int8      AS @Reqs,
            SUM(RoundedMetrics.Errs)::int8      AS @Errs,
            AVG(RoundedMetrics.Conns)::float4   AS @Conns,
            SUM(RoundedMetrics.Events)::int8    AS @Events,
            AVG(RoundedMetrics.P50)::float4     AS @P50,
            AVG(RoundedMetrics.P95)::float4     AS @P95,
            AVG(RoundedMetrics.P99)::float4     AS @P99
        FROM RoundedMetrics
        GROUP BY RoundedMetrics.RoundedTs
        ORDER BY RoundedMetrics.RoundedTs DESC
        LIMIT 100
    }).await?;

    Ok(stream.map(|row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => Ok((
            // key
            (Timestamp::UNIX_EPOCH + Duration::seconds(row.rounded_ts()?)).format_short(),
            // value
            AggregatedMetrics {
                mem: row.mem()?,
                upload: row.upload()?,
                reqs: row.reqs()?,
                errs: row.errs()?,
                conns: row.conns()?,
                events: row.events()?,
                p50: row.p50()?,
                p95: row.p95()?,
                p99: row.p99()?,
            },
        )),
    }))
}
