use std::sync::Arc;
use std::time::SystemTime;

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy, Serialize)]
pub struct FloatMetrics {
    pub mem: f32,
    pub upload: f32,
    pub reqs: f32,
    pub errs: f32,
    pub conns: f32,
    pub events: f32,
    pub p50: f32,
    pub p95: f32,
    pub p99: f32,
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
            mem: MEMORY_METRICS.allocated.get() as i64,
            upload: metrics.upload.get() as i64,
            reqs: count as i32,
            errs: metrics.errs.get() as i32,
            conns: state.gateway.conns.len() as i32,
            events: metrics.events.get() as i32,
            p50: p50 as i16,
            p95: p95 as i16,
            p99: p99 as i16,
        }
    }
}

use futures::{Stream, StreamExt};
use smol_str::SmolStr;

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

#[derive(Debug, Default, Clone, Deserialize)]
pub struct MetricsOptions {
    #[serde(default)]
    pub resolution: Option<u64>,

    #[serde(default)]
    pub start: Option<SmolStr>,
    #[serde(default)]
    pub end: Option<SmolStr>,
}

#[allow(deprecated)]
pub async fn get_metrics(
    state: ServerState,
    options: MetricsOptions,
) -> Result<impl Stream<Item = Result<(SmolStr, FloatMetrics), Error>>, Error> {
    let minute_resolution = match options.resolution {
        Some(res) if res > 5 => res as i64,
        _ => 5,
    };

    let secs = minute_resolution * 60;

    let start = options.start.and_then(|s| util::time::parse_iso8061(&s));
    let end = options.end.and_then(|s| util::time::parse_iso8061(&s));

    let db = state.db.read.get().await?;

    //#[rustfmt::skip]
    let stream = match (start, end) {
        (None, None) => db
            .query_stream_cached_typed(|| query(false, false), &[&secs])
            .await?
            .boxed(),
        (Some(start), Some(end)) => db
            .query_stream_cached_typed(|| query(true, true), &[&secs, &start, &end])
            .await?
            .boxed(),
        (Some(start), None) => db
            .query_stream_cached_typed(|| query(true, false), &[&secs, &start])
            .await?
            .boxed(),
        (None, Some(end)) => db
            .query_stream_cached_typed(|| query(false, true), &[&secs, &end])
            .await?
            .boxed(),
    };

    Ok(stream.map(|row| match row {
        Err(e) => Err(e.into()),
        Ok(row) => Ok((
            // key
            util::time::format_iso8061(time::PrimitiveDateTime::from_unix_timestamp(row.try_get(0)?)),
            // value
            FloatMetrics {
                mem: row.try_get(1)?,
                upload: row.try_get(2)?,
                reqs: row.try_get(3)?,
                errs: row.try_get(4)?,
                conns: row.try_get(5)?,
                events: row.try_get(6)?,
                p50: row.try_get(7)?,
                p95: row.try_get(8)?,
                p99: row.try_get(9)?,
            },
        )),
    }))
}

use thorn::*;

fn query(start: bool, end: bool) -> impl AnyQuery {
    use schema::*;

    const AVG_COLS: &[(Metrics, bool)] = &[
        (Metrics::Mem, false),
        (Metrics::Upload, true),
        (Metrics::Reqs, true),
        (Metrics::Errs, true),
        (Metrics::Conns, false),
        (Metrics::Events, true),
        (Metrics::P50, false),
        (Metrics::P95, false),
        (Metrics::P99, false),
    ];

    let resolution = Var::at(Type::INT8, 1);
    let first_ts = Var::at(Metrics::Ts, 2);
    let second_ts = Var::at(Metrics::Ts, 3);

    let rounded_ts = Builtin::round(
        Call::custom("date_part")
            .args((Literal::TextStr("epoch"), Metrics::Ts))
            .div(resolution.clone()),
    )
    .cast(Type::INT8) // ensures integer rounding
    .mul(resolution.clone())
    .rename_as("rounded_ts")
    .unwrap();

    let query = Query::select()
        .from_table::<Metrics>()
        .group_by(rounded_ts.reference())
        .expr(rounded_ts)
        .exprs(AVG_COLS.iter().map(|(col, use_sum)| {
            if *use_sum {
                Builtin::sum(*col)
            } else {
                Builtin::avg(*col)
            }
            .cast(Type::FLOAT4)
        }));

    match (start, end) {
        (false, false) => query,
        (true, false) => query.and_where(Metrics::Ts.greater_than_equal(first_ts)),
        (false, true) => query.and_where(Metrics::Ts.less_than(first_ts)),
        (true, true) => query.and_where(
            Metrics::Ts
                .greater_than_equal(first_ts)
                .and(Metrics::Ts.less_than(second_ts)),
        ),
    }
}
