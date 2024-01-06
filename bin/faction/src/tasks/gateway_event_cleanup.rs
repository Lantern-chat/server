use super::*;

pub fn add_gateway_event_cleanup_task(state: &ServerState, runner: &TaskRunner) {
    runner.add(RetryTask::new(IntervalFnTask::new(
        state.clone(),
        Duration::from_secs(60),
        |state, _| async move {
            log::trace!("Cleaning up gateway events");

            let mut lre = 0;

            for (_, conn) in state.gateway.gateways.iter(&scc::ebr::Guard::new()) {
                lre = lre.min(conn.last_event.load(Ordering::SeqCst));
            }

            // TODO: Replace with https://github.com/wvwwvwwv/scalable-concurrent-containers/issues/120 if added
            while lre > 0 && state.gateway.events.queue.remove_async(&lre).await {
                lre -= 1;
            }
        },
    )))
}
