pub mod api;
pub mod conn;
pub mod event;
pub mod handler;
pub mod heart;
pub mod structure;

use crate::prelude::*;

use self::event::EventInner;
pub use self::{event::Event, heart::Heart};

use std::sync::atomic::AtomicI64;
use std::time::Duration;

use hashbrown::HashMap;
use scc::ebr::Guard;
use sdk::Snowflake;
use tokio::sync::{broadcast, mpsc, Notify};
use triomphe::Arc;

pub type PartyId = Snowflake;
pub type UserId = Snowflake;
pub type EventId = Snowflake;
pub type ConnectionId = Snowflake;

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

#[derive(Default)]
pub struct Gateway {
    /// per-party emitters that can be subscribed to
    pub parties: scc::HashIndex<PartyId, PartyEmitter>,

    /// All gateway connections, even unidentified
    pub conns: scc::HashIndex<ConnectionId, GatewayConnection>,

    /// Identified gateway connections that can targetted by UserId
    pub users: scc::HashMap<UserId, HashMap<ConnectionId, GatewayConnection>>,

    /// Tracks connection heartbeats using a monotonic clock
    pub heart: Arc<Heart>,

    /// First element stores the actual last event, updated frequently
    ///
    /// Second element stores the last event 60 seconds ago as determined by the `event_cleanup` task.
    pub last_events: [AtomicI64; 2],

    /// Triggered by the database listener
    pub notifier: Notify,
}

impl ServerState {
    pub async fn new_gateway_connection(&self) -> (GatewayConnection, mpsc::Receiver<Event>) {
        let (conn, rx) = GatewayConnection::new(self);

        _ = self.gateway.conns.insert_async(conn.id, conn.clone()).await;

        (conn, rx)
    }
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

                        //crate::metrics::API_METRICS.load().errs.add(1);

                        // TODO: Better handling of this
                    }
                }
            })
            .await;

        //crate::metrics::API_METRICS.load().add_event();
    }

    pub fn broadcast_event(&self, event: Event, party_id: Snowflake) {
        match *event {
            EventInner::Internal(_) => log::debug!("broadcasting internal event"),
            EventInner::External(ref event) => {
                log::debug!("Sending event {:?} to party tx: {party_id}", event.msg.opcode());
            }
        }

        let guard = Guard::new();

        let Some(party) = self.parties.peek(&party_id, &guard) else {
            log::warn!("Could not find tx for party {party_id}!");
            return;
        };

        if let Err(e) = party.tx.send(event) {
            //crate::metrics::API_METRICS.load().errs.add(1);

            log::error!("Could not broadcast to party: {e}");
        }

        //crate::metrics::API_METRICS.load().add_event();
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
            let guard = scc::ebr::Guard::new();

            for &party_id in party_ids {
                if let Some(party) = self.parties.peek(&party_id, &guard) {
                    subs.push(party.subscribe())
                } else {
                    missing.push(party_id);
                }
            }
        }

        // this is really only invoked on startup or new parties
        for party_id in missing {
            subs.push(match self.parties.entry_async(party_id).await {
                scc::hash_index::Entry::Occupied(party) => party.get().subscribe(),
                scc::hash_index::Entry::Vacant(vacant) => vacant.insert_entry(PartyEmitter::new(party_id)).get().subscribe(),
            });
        }

        subs
    }
}
