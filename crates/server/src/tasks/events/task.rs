use std::sync::Arc;

use crate::ctrl::Error;
use crate::ServerState;

use db::pool::Object;
use failsafe::futures::CircuitBreaker;
use failsafe::{Config, Error as Reject};

use tokio::{
    sync::Notify,
    time::{Duration, Instant, Sleep},
};

pub async fn start(state: ServerState) {
    let circuit_breaker = Config::new().build();

    // store this outside the retry loop so if it loses connection while the server is running,
    // it doesn't lose track of where it left off while reconnecting.
    let mut latest_event = 0;

    while state.is_alive() {
        match circuit_breaker
            .call(async { event_loop(&state, &mut latest_event).await })
            .await
        {
            Ok(db) => db,
            Err(Reject::Inner(e)) => {
                log::error!("Error running event loop: {}", e);
                continue;
            }
            Err(Reject::Rejected) => {
                log::warn!("Event-loop has been rate-limited");
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };
    }
}

pub async fn event_loop(state: &ServerState, latest_event: &mut i64) -> Result<(), Error> {
    let db = state.db.read.get().await?;
    let mut listener = Object::take(state.db.read.get().await?);

    let row = db
        .query_opt_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<EventLog>()
                    .col(EventLog::Id)
                    .limit_n(1)
                    .order_by(EventLog::Id.descending())
            },
            &[],
        )
        .await?;

    if let Some(row) = row {
        *latest_event = row.try_get(0)?;
    }

    listener.execute("LISTEN event_log", &[]).await?;

    let notify = Arc::new(Notify::new());
    let listener_notify = notify.clone();
    let forwarding_subtask = tokio::spawn(async move {
        while let Some(_) = listener.recv_notif().await {
            listener_notify.notify_one();
        }
    });

    const DEBOUNCE_PERIOD: Duration = Duration::from_millis(100);

    let sleep = tokio::time::sleep(DEBOUNCE_PERIOD);
    futures::pin_mut!(sleep);

    let mut is_sleeping = true;

    while state.is_alive() {
        tokio::select! {
            biased;
            _ = &mut sleep, if is_sleeping => {
                is_sleeping = false;
            },
            _ = notify.notified() => {
                is_sleeping = true;
                sleep.as_mut().reset(Instant::now() + DEBOUNCE_PERIOD);
            },
        }

        // TODO: Read new events
    }

    let _ = forwarding_subtask.await;

    Ok(())
}
