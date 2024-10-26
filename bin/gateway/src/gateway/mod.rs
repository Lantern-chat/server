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
use tokio::sync::{broadcast, mpsc, Notify};
use triomphe::Arc;

/// Stored in the gateway, provides a channel directly
/// to a user connection
pub struct ConnectionEmitter {
    pub tx: mpsc::UnboundedSender<Event>,
}

/// Receives per-user events
pub struct ConnectionSubscription {
    pub rx: mpsc::UnboundedReceiver<Event>,
}

pub struct GenericSubscription {
    pub id: Snowflake, // PartyId or RoomId currently
    pub rx: broadcast::Receiver<Event>,
}

/// Stored in the gateway, provides a channel to room or party subscribers
#[derive(Clone)]
pub struct GenericEmitter {
    pub id: Snowflake,
    pub tx: mpsc::UnboundedSender<Event>,
    pub bc: broadcast::Sender<Event>,
}

impl GenericEmitter {
    pub fn new(id: Snowflake) -> Self {
        let bc = broadcast::channel(16).0;
        let (tx, mut rx) = mpsc::unbounded_channel();

        // TODO: Replace this mpsc buffering with database pulling?
        tokio::spawn({
            let bc = bc.clone();

            async move {
                while let Some(mut event) = rx.recv().await {
                    'try_loop: loop {
                        event = match bc.send(event) {
                            Ok(_) => break 'try_loop,
                            // If the error was because there are no receivers,
                            // just ignore the event entirely.
                            Err(_) if bc.receiver_count() == 0 => break 'try_loop,
                            // Move value back and retry
                            Err(broadcast::error::SendError(event)) => event,
                        };

                        tokio::time::sleep(Duration::from_millis(5)).await;
                    }
                }
            }
        });

        GenericEmitter { id, bc, tx }
    }

    pub fn subscribe(&self) -> GenericSubscription {
        GenericSubscription {
            id: self.id,
            rx: self.bc.subscribe(),
        }
    }
}

use conn::GatewayConnection;

#[derive(Default)]
pub struct Gateway {
    /// per-party emitters that can be subscribed to
    pub parties: scc::HashIndex<PartyId, GenericEmitter, sdk::FxRandomState2>,
    /// per-room emitters that can be subscribed to
    pub rooms: scc::HashIndex<RoomId, GenericEmitter, sdk::FxRandomState2>,

    /// All gateway connections, even unidentified
    pub conns: scc::HashIndex<ConnectionId, GatewayConnection, sdk::FxRandomState2>,

    /// Identified gateway connections that can targetted by UserId
    pub users: scc::HashMap<UserId, HashMap<ConnectionId, GatewayConnection>, sdk::FxRandomState2>,

    /// Tracks connection heartbeats using a monotonic clock
    pub heart: Arc<Heart>,

    /// First element stores the actual last event, updated frequently
    ///
    /// Second element stores the last event 60 seconds ago as determined by the `event_cleanup` task.
    pub last_events: [AtomicI64; 2],

    /// Triggered by the database listener
    pub notifier: Notify,

    pub structure: structure::StructureCache,
}

impl GatewayServerState {
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

    pub async fn broadcast_user_event(&self, event: Event, user_id: UserId) {
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

    pub fn broadcast_event(&self, event: Event, party_id: PartyId) {
        match *event {
            EventInner::Internal(_) => log::debug!("broadcasting internal event"),
            EventInner::External(ref event) => {
                log::debug!("Sending event {:?} to party tx: {party_id}", event.msg.opcode());
            }
        }

        let guard = scc::ebr::Guard::new();

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
        party_ids: impl IntoIterator<Item = PartyId>,
        room_ids: impl IntoIterator<Item = RoomId>,
    ) -> Subscriptions {
        let (_, subs) = tokio::join!(
            self.activate_connection(user_id, conn),
            self.subscribe(party_ids, room_ids),
        );

        subs
    }

    pub async fn remove_connection(&self, conn_id: ConnectionId, user_id: Option<UserId>) {
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

    async fn subscribe(&self, party_ids: impl IntoIterator<Item = PartyId>, room_ids: impl IntoIterator<Item = RoomId>) -> Subscriptions {
        let mut subs = Subscriptions::default();

        let mut missing_parties = Vec::new();
        let mut missing_rooms = Vec::new();

        {
            let guard = scc::ebr::Guard::new();

            for party_id in party_ids {
                if let Some(party) = self.parties.peek(&party_id, &guard) {
                    subs.parties.push(party.subscribe())
                } else {
                    missing_parties.push(party_id);
                }
            }

            for room_id in room_ids {
                if let Some(room) = self.rooms.peek(&room_id, &guard) {
                    subs.rooms.push(room.subscribe())
                } else {
                    missing_rooms.push(room_id);
                }
            }
        }

        // this is really only invoked on startup or new parties/rooms
        tokio::join! {
            async {
                for party_id in missing_parties {
                    subs.parties.push(match self.parties.entry_async(party_id).await {
                        scc::hash_index::Entry::Occupied(party) => party.get().subscribe(),
                        scc::hash_index::Entry::Vacant(vacant) => vacant.insert_entry(GenericEmitter::new(party_id)).get().subscribe(),
                    });
                }
            },
            async {
                for room_id in missing_rooms {
                    subs.rooms.push(match self.rooms.entry_async(room_id).await {
                        scc::hash_index::Entry::Occupied(room) => room.get().subscribe(),
                        scc::hash_index::Entry::Vacant(vacant) => vacant.insert_entry(GenericEmitter::new(room_id)).get().subscribe(),
                    });
                }
            },
        };

        subs
    }
}

#[derive(Default)]
pub struct Subscriptions {
    pub parties: Vec<GenericSubscription>,
    pub rooms: Vec<GenericSubscription>,
}
