use std::sync::Arc;

use crate::ctrl::Error;
use crate::ServerState;

use db::pg::AsyncMessage;
use db::pool::Object;
use schema::{EventCode, Snowflake};

use failsafe::futures::CircuitBreaker;
use failsafe::{Config, Error as Reject};

use hashbrown::HashMap;

use futures::{Stream, StreamExt, TryFutureExt};

use tokio::{
    sync::Notify,
    time::{Duration, Instant, Sleep},
};

use super::RawEvent;

pub async fn start(state: ServerState) {
    let circuit_breaker = Config::new().build();

    // store this outside the retry loop so if it loses connection while the server is running,
    // it doesn't lose track of where it left off while reconnecting.
    let mut latest_event = 0;

    while state.is_alive() {
        // as async {} blocks are lazy, only call `event_loop` within one, so rejections don't invoke it.
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
                use schema::*;
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

    // when `kill` is dropped, `dead` will be resolved

    // signifies if the query loop is dead
    let (query_kill, query_dead) = tokio::sync::oneshot::channel::<()>();
    // signifies if the event loop is dead
    let (event_kill, event_dead) = tokio::sync::oneshot::channel::<()>();

    // repeatable data-less notification
    let event_notify = Arc::new(Notify::new());
    let event_notify2 = event_notify.clone();

    // begin listening on current task to allow it to fail gracefully
    listener.execute("LISTEN event_log", &[]).await?;

    let forwarding_subtask = tokio::spawn(async move {
        futures::pin_mut!(query_dead);

        let conn = listener.take_connection().await;

        let mut stream = conn.stream.lock().await;

        // receive and notify of events until kill signal is received
        loop {
            let event = tokio::select! {
                biased;
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

        // trigger kill of event loop ASAP
        drop(event_kill);
        // free stream lock
        drop(stream);
        // drop client
        drop(listener);

        // ensure connection is dropped
        assert_eq!(Arc::strong_count(&conn.stream), 1);
        drop(conn);

        log::info!("Disconnected from listener connection");
    });

    // Note that because futures are lazy, this does nothing until awaited upon
    let stop_subtask = async {
        drop(query_kill); // trigger exit of notification subtask

        log::info!("Waiting on event notification subtask to complete...");
        let _ = forwarding_subtask.await;
    };

    let mut party_events: HashMap<Snowflake, Vec<RawEvent>> = HashMap::new();
    let mut direct_events: HashMap<Snowflake, Vec<RawEvent>> = HashMap::new();
    let mut user_events: Vec<RawEvent> = Vec::new();

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

        // wrap in async block to coalesce errors to be handled below
        let res = async {
            let stream = db
                .query_stream_cached_typed(
                    || {
                        use schema::*;
                        use thorn::*;

                        Query::select()
                            .from_table::<EventLog>()
                            .cols(&[
                                EventLog::Counter,
                                EventLog::Code,
                                EventLog::Id,
                                EventLog::PartyId,
                                EventLog::RoomId,
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
            if let Some(mut row_res) = stream.next().await {
                loop {
                    let row = row_res?;

                    let event = RawEvent {
                        code: row.try_get(1)?,
                        id: row.try_get(2)?,
                        room_id: row.try_get(4)?,
                    };

                    match row.try_get(3)? {
                        Some(party_id) => party_events.entry(party_id).or_default().push(event),
                        None => match event.room_id {
                            None => user_events.push(event),
                            Some(room_id) => direct_events.entry(room_id).or_default().push(event),
                        },
                    }

                    row_res = match stream.next().await {
                        Some(row) => row,
                        None => {
                            // defer parsing this field until it's the last event
                            next_latest_event = row.try_get(0)?;
                            break;
                        }
                    };
                }
            }

            log::debug!(
                "Received {} party_events, {} direct events, {} user_events",
                party_events.len(),
                direct_events.len(),
                user_events.len()
            );

            // process events from each party in parallel,
            // but within each party process them sequentially
            async fn process_party_events(
                events: &mut HashMap<Snowflake, Vec<RawEvent>>,
                state: &ServerState,
                db: &db::pool::Client,
            ) {
                futures::stream::iter(events.drain())
                    .for_each_concurrent(state.config.num_parallel_tasks, |(party_id, events)| async move {
                        for event in events {
                            if let Err(e) = super::process(&state, db, event, Some(party_id)).await {
                                log::error!("Error processing party event: {:?} {}", event, e);
                                // TODO: Disconnect party
                            }
                        }
                    })
                    .await
            }

            // process events from each direct-room in parallel,
            // but within each room process them sequentially
            async fn process_direct_events(
                events: &mut HashMap<Snowflake, Vec<RawEvent>>,
                state: &ServerState,
                db: &db::pool::Client,
            ) {
                futures::stream::iter(events.drain())
                    .for_each_concurrent(state.config.num_parallel_tasks, |(_room_id, events)| async move {
                        for event in events {
                            if let Err(e) = super::process(&state, db, event, None).await {
                                log::error!("Error processing direct event: {:?} {}", event, e);
                                // TODO: Disconnect users
                            }
                        }
                    })
                    .await
            }

            // user events can be processed in any order
            async fn process_user_events(
                events: &mut Vec<RawEvent>,
                state: &ServerState,
                db: &db::pool::Client,
            ) {
                futures::stream::iter(events.drain(..))
                    .for_each_concurrent(state.config.num_parallel_tasks, |event| async move {
                        if let Err(e) = super::process(&state, db, event, None).await {
                            log::error!("Error processing user event: {:?} {}", event, e);
                            // TODO: Disconnect users
                        }
                    })
                    .await
            }

            tokio::join!(
                process_party_events(&mut party_events, state, &db),
                process_direct_events(&mut direct_events, state, &db),
                process_user_events(&mut user_events, state, &db),
            );

            *latest_event = next_latest_event;

            Ok(())
        };

        // if there is an error, ensure the forwarding subtask exits first
        if let Err(e) = res.await {
            stop_subtask.await;
            return Err(e);
        }
    }

    stop_subtask.await;

    Ok(())
}
