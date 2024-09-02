use futures::{StreamExt, TryStreamExt};
use hashbrown::HashMap;
use std::sync::atomic::Ordering;
use tokio::time::{Duration, Instant};

use task_runner::{RetryAsyncFnTask, TaskRunner};

use db::Object;
use schema::Snowflake;

use super::event_processors::RawEvent;
use crate::prelude::*;

const DEBOUNCE_PERIOD: Duration = Duration::from_millis(100);

pub fn add_gateway_processor(state: ServerState, runner: &TaskRunner) {
    runner.add(RetryAsyncFnTask::new(state, |mut alive, state| async move {
        let db = Object::take(state.db.read.get().await?);

        // if task has never run before, get latest event
        // otherwise it's assumed to be resuming from a crashed iteration
        if state.gateway.last_event().load(Ordering::SeqCst) == 0 {
            #[rustfmt::skip]
            let row = db.query_opt2(schema::sql! {
                SELECT
                    EventLog.Counter AS @_
                FROM EventLog
                ORDER BY EventLog.Counter DESC
                LIMIT 1
            }).await?;

            if let Some(row) = row {
                state.gateway.last_event().store(row.event_log_counter()?, Ordering::SeqCst);
            }
        }

        let mut sleep = std::pin::pin!(tokio::time::sleep(DEBOUNCE_PERIOD));
        let mut is_sleeping = true;

        let mut party_events: HashMap<Snowflake, Vec<RawEvent>> = HashMap::new();
        let mut direct_events: HashMap<Snowflake, Vec<RawEvent>> = HashMap::new();
        let mut user_events: Vec<RawEvent> = Vec::new();

        loop {
            tokio::select! {
                biased;
                // always check every `DEBOUNCE_PERIOD` interval
                _ = &mut sleep, if is_sleeping => {
                    is_sleeping = false;
                },
                // rely on notifications for instant checks
                _ = state.gateway.notifier.notified() => {
                    // just in case, check `DEBOUNCE_PERIOD` from now for any stragglers
                    is_sleeping = true;
                    sleep.as_mut().reset(Instant::now() + DEBOUNCE_PERIOD);
                },
                _ = alive.changed() => break,
            }

            let mut latest_event = state.gateway.last_event().load(Ordering::SeqCst);

            #[rustfmt::skip]
            let stream = db.query_stream2(schema::sql! {
                const_assert!(!Columns::IS_DYNAMIC);

                SELECT
                    EventLog.Counter AS @_,
                    EventLog.Code AS @_,
                    EventLog.Id AS @_,
                    EventLog.PartyId AS @_,
                    EventLog.RoomId AS @_
                FROM EventLog
                WHERE EventLog.Counter > #{&latest_event as EventLog::Counter}
                ORDER BY EventLog.Counter ASC
            }).await?;

            // partition events by party or generic user events
            let mut stream = std::pin::pin!(stream);

            while let Some(row) = stream.try_next().await? {
                let event = RawEvent {
                    code: row.event_log_code()?,
                    id: row.event_log_id()?,
                    room_id: row.event_log_room_id()?,
                };

                match row.event_log_party_id()? {
                    Some(party_id) => party_events.entry(party_id).or_default().push(event),
                    None => match event.room_id {
                        None => user_events.push(event),
                        Some(room_id) => direct_events.entry(room_id).or_default().push(event),
                    },
                }

                latest_event = row.event_log_counter()?;
            }

            log::debug!(
                "Received {} party_events, {} direct events, {} user_events",
                party_events.len(),
                direct_events.len(),
                user_events.len(),
            );

            // process events from each party in parallel,
            // but within each party process them sequentially
            async fn process_party_events(
                events: &mut HashMap<Snowflake, Vec<RawEvent>>,
                state: &ServerState,
                db: &db::Client,
            ) {
                futures::stream::iter(events.drain())
                    .for_each_concurrent(None, |(party_id, events)| async move {
                        for event in events {
                            if let Err(e) =
                                super::event_processors::process(state, db, event, Some(party_id)).await
                            {
                                log::error!("Error processing party event: {event:?} {e}");
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
                db: &db::Client,
            ) {
                futures::stream::iter(events.drain())
                    .for_each_concurrent(None, |(_room_id, events)| async move {
                        for event in events {
                            if let Err(e) = super::event_processors::process(state, db, event, None).await {
                                log::error!("Error processing direct event: {event:?} {e}");
                                // TODO: Disconnect users
                            }
                        }
                    })
                    .await
            }

            // user events can be processed in any order
            async fn process_user_events(events: &mut Vec<RawEvent>, state: &ServerState, db: &db::Client) {
                futures::stream::iter(events.drain(..))
                    .for_each_concurrent(None, |event| async move {
                        if let Err(e) = super::event_processors::process(state, db, event, None).await {
                            log::error!("Error processing user event: {event:?} {e}");
                            // TODO: Disconnect users
                        }
                    })
                    .await
            }

            tokio::join!(
                process_party_events(&mut party_events, &state, &db),
                process_direct_events(&mut direct_events, &state, &db),
                process_user_events(&mut user_events, &state, &db),
            );

            state.gateway.last_event().store(latest_event, Ordering::SeqCst);
        }

        Ok::<(), Error>(())
    }))
}
