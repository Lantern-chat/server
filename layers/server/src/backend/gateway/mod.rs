pub mod conn;
pub mod event;
pub mod task;

use std::sync::{atomic::AtomicI64, Arc};
use std::{borrow::Cow, error::Error, pin::Pin, time::Duration};

use futures::{future, Future, FutureExt, SinkExt, StreamExt, TryStreamExt};

use hashbrown::HashMap;
use tokio::sync::{broadcast, mpsc, Notify, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

use schema::Snowflake;

pub type PartyId = Snowflake;
pub type UserId = Snowflake;
pub type EventId = Snowflake;
pub type ConnectionId = Snowflake;

pub use event::Event;

/// Stored in the gateway, provides a channel directly
/// to a user connection
pub struct ConnectionEmitter {
    pub tx: mpsc::UnboundedSender<Event>,
}

/// Receives per-user events
pub struct ConnectionSubscription {
    pub rx: mpsc::UnboundedReceiver<Event>,
}

/// Receives party events
pub struct PartySubscription {
    pub party_id: PartyId,
    pub rx: broadcast::Receiver<Event>,
}

/// Stored in the gateway, provides a channel to party subscribers
#[derive(Clone)]
pub struct PartyEmitter {
    pub id: PartyId,
    pub tx: mpsc::UnboundedSender<Event>,
    pub bc: broadcast::Sender<Event>,
}

impl PartyEmitter {
    pub fn new(id: PartyId) -> Self {
        let bc = broadcast::channel(16).0;
        let (tx, mut rx) = mpsc::unbounded_channel();

        let bc2 = bc.clone();

        // TODO: Replace this mpsc buffering with database pulling
        tokio::spawn(async move {
            while let Some(mut event) = rx.recv().await {
                'try_loop: loop {
                    event = match bc2.send(event) {
                        Ok(_) => break 'try_loop,
                        // If the error was because there are no receivers,
                        // just ignore the event entirely.
                        Err(_) if bc2.receiver_count() == 0 => break 'try_loop,
                        // Move value back and retry
                        Err(broadcast::error::SendError(event)) => event,
                    };

                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }
        });

        PartyEmitter { id, bc, tx }
    }

    pub fn subscribe(&self) -> PartySubscription {
        PartySubscription {
            party_id: self.id,
            rx: self.bc.subscribe(),
        }
    }
}

use conn::GatewayConnection;

use crate::backend::gateway::event::EventInner;

#[derive(Default)]
pub struct Gateway {
    /// per-party emitters that can be subscribed to
    pub parties: scc::HashIndex<PartyId, PartyEmitter>,

    /// All gateway connections, even unidentified
    pub conns: scc::HashIndex<ConnectionId, GatewayConnection>,

    /// Identified gateway connections that can targetted by UserId
    pub users: scc::HashMap<UserId, HashMap<ConnectionId, GatewayConnection>>,

    /// First element stores the actual last event, updated frequently
    ///
    /// Second element stores the last event 60 seconds ago as determined by the `event_cleanup` task.
    pub last_events: [AtomicI64; 2],

    /// Triggered by the database listener
    pub notifier: Notify,
}

impl Gateway {
    #[inline]
    pub fn last_event(&self) -> &AtomicI64 {
        &self.last_events[0]
    }

    pub async fn broadcast_user_event(&self, event: Event, user_id: Snowflake) {
        self.users
            .read_async(&user_id, |_, users| {
                for conn in users.values() {
                    if let Err(e) = conn.tx.try_send(event.clone()) {
                        log::warn!("Could not send message to user connection: {}", e);

                        conn.is_active.store(false, std::sync::atomic::Ordering::Relaxed);
                        conn.kill.notify_waiters();

                        crate::metrics::API_METRICS.load().errs.add(1);

                        // TODO: Better handling of this
                    }
                }
            })
            .await;

        crate::metrics::API_METRICS.load().add_event();
    }

    pub fn broadcast_event(&self, event: Event, party_id: Snowflake) {
        let sent = self.parties.read(&party_id, |_, party| {
            match *event {
                EventInner::External(ref event) => {
                    log::debug!("Sending event {:?} to party tx: {party_id}", event.msg.opcode());
                }
                EventInner::Internal(_) => log::debug!("broadcasting internal event"),
            }

            if let Err(e) = party.tx.send(event) {
                crate::metrics::API_METRICS.load().errs.add(1);

                log::error!("Could not broadcast to party: {e}");
            }
        });

        if sent.is_none() {
            log::warn!("Could not find party {party_id}!");
        }

        crate::metrics::API_METRICS.load().add_event();
    }

    /// After identifying, a connection can be added to active subscriptions
    #[rustfmt::skip]
    pub async fn sub_and_activate_connection(
        &self,
        user_id: UserId,
        conn: GatewayConnection,
        party_ids: impl IntoIterator<Item = &PartyId>,
    ) -> Vec<PartySubscription> {
        let (_, subs) = tokio::join!(
            self.activate_connection(user_id, conn),
            self.subscribe(party_ids),
        );

        subs
    }

    pub async fn add_connection(&self, conn: GatewayConnection) {
        _ = self.conns.insert_async(conn.id, conn).await;
    }

    pub async fn remove_connection(&self, conn_id: Snowflake, user_id: Option<Snowflake>) {
        tokio::join!(self.conns.remove_async(&conn_id), async {
            if let Some(user_id) = user_id {
                if let scc::hash_map::Entry::Occupied(mut occupied) = self.users.entry_async(user_id).await {
                    let user = occupied.get_mut();
                    user.remove(&conn_id);
                    if user.is_empty() {
                        _ = occupied.remove();
                    }
                }
            }
        });
    }

    async fn activate_connection(&self, user_id: UserId, conn: GatewayConnection) {
        self.users.entry_async(user_id).await.or_default().get_mut().insert(conn.id, conn);
    }

    async fn subscribe(&self, party_ids: impl IntoIterator<Item = &PartyId>) -> Vec<PartySubscription> {
        let mut subs = Vec::new();
        let mut missing = Vec::new();

        {
            let barrier = scc::ebr::Barrier::new();

            for &party_id in party_ids {
                if self.parties.read_with(&party_id, |_, party| subs.push(party.subscribe()), &barrier).is_none() {
                    missing.push(party_id);
                }
            }
        }

        // this is really only invoked on startup
        for party_id in missing {
            let party = PartyEmitter::new(party_id);
            let mut sub = party.subscribe();

            // if this errors, then a race-condition occurred and we should just retry reading the party emitter
            if self.parties.insert_async(party_id, party).await.is_err() {
                if let Some(new_sub) = self.parties.read(&party_id, |_, party| party.subscribe()) {
                    // forget the old sub and party
                    sub = new_sub;
                } else {
                    panic!("Inconsistent state of gateway party emitters");
                }
            }

            subs.push(sub);
        }

        subs
    }
}
