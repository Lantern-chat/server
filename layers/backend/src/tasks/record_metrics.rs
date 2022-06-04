use crate::api::metrics::Metrics;

use super::*;

pub fn add_record_metrics_task(state: &State, runner: &TaskRunner) {
    runner.add(task_runner::interval_fn_task(
        state.clone(),
        Duration::from_secs(60 * 5),
        |_, state| async {
            log::trace!("Collecting metrics");

            let task = async {
                let db = state.db.write.get().await?;

                let metrics = Metrics::acquire(state);

                db.execute_cached_typed(
                    || {
                        use schema::*;
                        use thorn::*;

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
                    },
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

                Ok::<(), crate::Error>(())
            };

            if let Err(e) = task.await {
                log::error!("Error collecting metrics! {e}");
            }
        },
    ))
}
