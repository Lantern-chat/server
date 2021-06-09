pub mod conn;
pub mod event;
pub mod models;
pub mod msg;
pub mod socket;

use std::{borrow::Cow, error::Error, pin::Pin, time::Duration};

use futures::{future, Future, FutureExt, SinkExt, StreamExt, TryStreamExt};

use hashbrown::HashMap;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;

use db::Snowflake;
use util::cmap::CHashMap;

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
    pub parties: CHashMap<PartyId, PartyEmitter>,

    /// All gateway connections, even unidentified
    pub conns: CHashMap<ConnectionId, GatewayConnection>,

    /// Identified gateway connections that can targetted by UserId
    pub users: CHashMap<UserId, HashMap<ConnectionId, GatewayConnection>>,
}

impl Gateway {
    pub async fn broadcast_event(&self, event: Event, id: Snowflake, user: bool) {
        if user {
            if let Some(users) = self.users.get(&id).await {
                for conn in users.values() {
                    if let Err(e) = conn.tx.try_send(event.clone()) {
                        log::warn!("Could not send message to user connection: {}", e);
                        conn.is_active.store(false, std::sync::atomic::Ordering::Relaxed);
                        conn.kill.notify_waiters();

                        // TODO: Better handling of this
                    }
                }
            }
        } else {
            if let Some(party) = self.parties.get(&id).await {
                log::info!("Sending event to party tx: {}", id);
                if let Err(e) = party.tx.send(event) {
                    log::error!("Could not broadcast to party: {}", e);
                }
            } else {
                log::warn!("Could not find party {}!", id);
            }
        }
    }

    /// After identifying, a connection can be added to active subscriptions
    #[rustfmt::skip]
    pub async fn sub_and_activate_connection(
        &self,
        user_id: UserId,
        conn: GatewayConnection,
        party_ids: impl IntoIterator<Item = &PartyId>,
    ) -> Vec<PartySubscription> {
        let (_, subs) = futures::future::join(
            self.activate_connection(user_id, conn),
            self.subscribe(party_ids)).await;

        subs
    }

    pub async fn add_connection(&self, conn: GatewayConnection) {
        self.conns.insert(conn.id, conn).await;
    }

    async fn activate_connection(&self, user_id: UserId, conn: GatewayConnection) {
        self.users
            .get_mut_or_default(&user_id)
            .await
            .insert(conn.id, conn);
    }

    async fn subscribe(&self, party_ids: impl IntoIterator<Item = &PartyId>) -> Vec<PartySubscription> {
        let mut subs = Vec::new();
        let mut missing = Vec::new();
        let mut cache = Vec::new();

        self.parties
            .batch_read(
                party_ids.into_iter(),
                Some(&mut cache),
                |key, value| match value {
                    Some((_, p)) => subs.push(p.subscribe()),
                    None => missing.push(key),
                },
            )
            .await;

        if !missing.is_empty() {
            self.parties
                .batch_write(missing, Some(&mut cache), |key, value| {
                    log::info!("Added gateway entry for missing party: {}", key);
                    subs.push(
                        value
                            .or_insert_with(|| (*key, PartyEmitter::new(*key)))
                            .1
                            .subscribe(),
                    )
                })
                .await;
        }

        subs
    }
}
