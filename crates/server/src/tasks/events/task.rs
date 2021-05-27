use std::sync::Arc;

use crate::ctrl::Error;
use crate::ServerState;

use db::pg::AsyncMessage;
use db::pool::Object;
use db::Snowflake;

use failsafe::futures::CircuitBreaker;
use failsafe::{Config, Error as Reject};

use hashbrown::HashMap;

use futures::{Stream, StreamExt, TryFutureExt};

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
    log::info!("Starting event-log loop");

    // Acquire two dedicated database connections from the read-only pool
    let db = Object::take(state.db.read.get().await?);
    let listener = Object::take(state.db.read.get().await?);

    // start off by getting the most-recent counter value

    let row = db
        .query_opt_cached_typed(
            || {
                use db::schema::*;
                use thorn::*;

                Query::select()
                    .from_table::<EventLog>()
                    .col(EventLog::Counter)
                    .limit_n(1)
                    .order_by(EventLog::Counter.descending())
            },
            &[],
        )
        .await?;

    if let Some(row) = row {
        *latest_event = row.try_get(0)?;
    }

    // begin listening on current task to allow it to fail gracefully
    listener.execute("LISTEN event_log", &[]).await?;

    // when `kill` is dropped, `dead` will be resolved

    // signifies if the query loop is dead
    let (query_kill, query_dead) = tokio::sync::oneshot::channel::<()>();
    // signifies if the event loop is dead
    let (event_kill, event_dead) = tokio::sync::oneshot::channel::<()>();

    // repeatable data-less notification
    let event_notify = Arc::new(Notify::new());

    let event_notify2 = event_notify.clone();
    let forwarding_subtask = tokio::spawn(async move {
        futures::pin_mut!(query_dead);

        let conn = listener.take_connection().await;

        let mut stream = conn.stream.lock().await;

        // receive and notify of events until kill signal is received
        loop {
            let event = tokio::select! {
                //biased;
                event = stream.next() => { event }
                _ = &mut query_dead => { break; }
            };

            match event {
                Some(Ok(AsyncMessage::Notification(_))) => {
                    event_notify2.notify_one();
                }
                Some(Ok(AsyncMessage::Notice(notice))) => {
                    log::info!("Database notice: {}", notice);
                }
                Some(Ok(_)) => unreachable!("AsyncMessage is non-exhaustive"),
                Some(Err(e)) => {
                    log::error!("Database connection error: {}", e);

                    // if there is an error, it can't be recovered from, so drop the connection
                    return;
                }
                None => break,
            }
        }

        drop(event_kill);
    });

    #[derive(Debug, Clone, Copy)]
    pub struct EventCode {
        pub id: Snowflake,
        pub code: i16,
    }

    let mut party_events: HashMap<Snowflake, Vec<EventCode>> = HashMap::new();
    let mut user_events: Vec<EventCode> = Vec::new();

    const DEBOUNCE_PERIOD: Duration = Duration::from_millis(100);

    let sleep = tokio::time::sleep(DEBOUNCE_PERIOD);
    let mut is_sleeping = true;

    futures::pin_mut!(sleep);
    futures::pin_mut!(event_dead);

    loop {
        tokio::select! {
            biased; // way too much overhead calling thread_rng
            _ = &mut sleep, if is_sleeping => {
                is_sleeping = false;
            },
            _ = event_notify.notified() => {
                is_sleeping = true;
                sleep.as_mut().reset(Instant::now() + DEBOUNCE_PERIOD);
            },
            _ = &mut event_dead => { break; }
            _ = state.notify_shutdown.notified() => { break; }
        }

        let stream = db
            .query_stream_cached_typed(
                || {
                    use db::schema::*;
                    use thorn::*;

                    Query::select()
                        .from_table::<EventLog>()
                        .cols(&[
                            EventLog::Counter,
                            EventLog::Code,
                            EventLog::Id,
                            EventLog::PartyId,
                        ])
                        .order_by(EventLog::Counter.ascending())
                        .and_where(EventLog::Counter.greater_than(Var::of(EventLog::Counter)))
                },
                &[latest_event],
            )
            .await?;

        // track this separately so that if anything in the upcoming loop fails it doesn't leave the
        // latest_event in an incomplete state. Only assign it when everything has been completed.
        let mut next_latest_event = *latest_event;

        // partition events by party or generic user events
        futures::pin_mut!(stream);
        while let Some(row) = stream.next().await {
            let row = row?;

            next_latest_event = row.try_get(0)?;

            let event = EventCode {
                code: row.try_get(1)?,
                id: row.try_get(2)?,
            };

            match row.try_get(3)? {
                Some(party_id) => party_events.entry(party_id).or_default().push(event),
                None => user_events.push(event),
            }
        }

        log::info!("{:?} {:?}", party_events.len(), user_events.len());

        party_events.clear();
        user_events.clear();

        *latest_event = next_latest_event;
    }

    drop(query_kill);

    log::info!("Waiting on event notification subtask to complete...");
    let _ = forwarding_subtask.await;

    Ok(())
}
