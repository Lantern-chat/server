use crate::backend::api::metrics::Metrics;

use super::*;

pub fn add_record_metrics_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(60 * 5),
        |state, _| async move {
            log::trace!("Collecting metrics");

            let task = async {
                let Some(metrics) = Metrics::acquire(&state) else { return Ok(()); };

                #[rustfmt::skip]
                state.db.write.get().await?.execute2(schema::sql! {
                    INSERT INTO Metrics (Mem, Upload, Reqs, Errs, Conns, Events, P50, P95, P99) VALUES (
                        #{&metrics.mem      as Metrics::Mem},
                        #{&metrics.upload   as Metrics::Upload},
                        #{&metrics.reqs     as Metrics::Reqs},
                        #{&metrics.errs     as Metrics::Errs},
                        #{&metrics.conns    as Metrics::Conns},
                        #{&metrics.events   as Metrics::Events},
                        #{&metrics.p50      as Metrics::P50},
                        #{&metrics.p95      as Metrics::P95},
                        #{&metrics.p99      as Metrics::P99}
                    )
                }).await?;

                Ok::<(), crate::Error>(())
            };

            if let Err(e) = task.await {
                log::error!("Error collecting metrics! {e}");
            }
        },
    )))
}
