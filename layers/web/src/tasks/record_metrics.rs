use std::time::Duration;

use crate::ctrl::{metrics::Metrics, Error};
use crate::ServerState;

pub async fn record_metrics(state: ServerState) {
    let mut interval = tokio::time::interval(Duration::from_secs(60 * 5));

    loop {
        tokio::select! {
            biased;
            _ = interval.tick() => {},
            _ = state.notify_shutdown.notified() => { break; }
        };

        log::trace!("Collecting metrics");

        let task = async {
            let db = state.db.write.get().await?;

            let metrics = Metrics::acquire(&state);

            db.execute_cached_typed(
                || query(),
                &[
                    &metrics.mem,
                    &metrics.upload,
                    &metrics.reqs,
                    &metrics.errs,
                    &metrics.conns,
                    &metrics.events,
                    &metrics.p50,
                    &metrics.p95,
                    &metrics.p99,
                ],
            )
            .await?;

            Ok::<(), Error>(())
        };

        if let Err(e) = task.await {
            log::error!("Error collecting metrics! {e}");
        }
    }
}

use thorn::*;

fn query() -> impl AnyQuery {
    use schema::*;

    const COLS: &[Metrics] = &[
        Metrics::Mem,
        Metrics::Upload,
        Metrics::Reqs,
        Metrics::Errs,
        Metrics::Conns,
        Metrics::Events,
        Metrics::P50,
        Metrics::P95,
        Metrics::P99,
    ];

    Query::insert()
        .into::<Metrics>()
        .cols(COLS)
        .values(COLS.iter().copied().map(Var::of))
}
