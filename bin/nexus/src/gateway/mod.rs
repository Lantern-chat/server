#![allow(unused_labels)]

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

use crate::prelude::*;

pub type ConnectionId = Snowflake;

use sdk::api::error::ApiError;

use tokio::sync::Notify;
use triomphe::Arc;

use framed::tokio::AsyncFramedWriter;
use rkyv::{
    ser::{serializers::AllocSerializer, Serializer},
    AlignedVec, Archive, Serialize,
};

use quinn::{Connection, ConnectionError, RecvStream, SendStream, VarInt};

pub mod rpc;
pub mod task;

#[derive(Clone)]
pub struct RpcConnection {
    pub id: ConnectionId,
    pub conn: Connection,
}

pub struct GatewayConnectionInner {
    pub id: ConnectionId,
    pub conn: Connection,
    pub last_event: AtomicU64,
}

#[derive(Clone)]
pub struct GatewayConnection(Arc<GatewayConnectionInner>);

impl core::ops::Deref for GatewayConnection {
    type Target = GatewayConnectionInner;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct EventQueue {
    /// Increments for each new event
    pub counter: AtomicU64,
    /// Notifies when a new event has been added to the queue
    pub notify: Notify,
    /// Holds the events and allows concurrent access
    pub queue: scc::TreeIndex<u64, Arc<AlignedVec>>,
}

impl EventQueue {
    #[rustfmt::skip]
    pub fn batch_since(&self, cnt: u64) -> Vec<(u64, Arc<AlignedVec>)> {
        const EVENT_BATCH_SIZE: usize = 64;

        self.queue.range(cnt.., &scc::ebr::Guard::new())
            .take(EVENT_BATCH_SIZE).map(|(&k, v)| (k, v.clone())).collect()
    }

    pub async fn send_simple<T>(&self, event: &T)
    where
        T: Archive + Serialize<AllocSerializer<512>>,
    {
        self.send::<T, 512>(event).await
    }

    pub async fn send<T, const N: usize>(&self, event: &T)
    where
        T: Archive + Serialize<AllocSerializer<N>>,
    {
        let mut serializer = AllocSerializer::<N>::default();

        if let Err(e) = serializer.serialize_value(event) {
            log::error!("Rkyv Encoding Error: {e}");
        }

        // TODO: Compression?
        _ = self
            .queue
            .insert_async(
                self.counter.fetch_add(1, Ordering::SeqCst),
                Arc::new(serializer.into_serializer().into_inner()),
            )
            .await;

        self.notify.notify_waiters();
    }
}

pub struct Gateway {
    pub events: EventQueue,

    /// RPC Connections
    pub rpcs: scc::HashIndex<ConnectionId, RpcConnection>,
    /// Gateway Stream Connections
    pub gateways: scc::HashIndex<ConnectionId, GatewayConnection>,

    /// Triggered by the database listener in the listener task
    pub notifier: Notify,

    /// First element stores the actual last event, updated frequently
    ///
    /// Second element stores the last event 60 seconds ago as determined by the `event_cleanup` task.
    pub last_events: [AtomicI64; 2],
}

impl Default for EventQueue {
    fn default() -> Self {
        EventQueue {
            counter: AtomicU64::new(1), // zero index might cause issues with some logic
            notify: Notify::new(),
            queue: Default::default(),
        }
    }
}

impl Default for Gateway {
    fn default() -> Self {
        Gateway {
            events: Default::default(),
            rpcs: Default::default(),
            gateways: Default::default(),
            notifier: Notify::new(),
            last_events: Default::default(),
        }
    }
}

impl Gateway {
    #[inline]
    pub fn last_event(&self) -> &AtomicI64 {
        &self.last_events[0]
    }

    pub async fn insert_rpc_connection(&self, state: ServerState, conn: Connection) {
        let conn = RpcConnection {
            id: state.sf.gen(),
            conn,
        };

        tokio::spawn(conn.clone().run_rpc(state));

        _ = self.rpcs.insert_async(conn.id, conn).await;
    }

    pub async fn insert_gateway_connection(&self, state: ServerState, conn: Connection) {
        let conn = GatewayConnection(Arc::new(GatewayConnectionInner {
            id: state.sf.gen(),
            conn,
            last_event: AtomicU64::new(0),
        }));

        tokio::spawn(conn.clone().run_gateway(state));

        _ = self.gateways.insert_async(conn.id, conn).await;
    }
}

use failsafe::{futures::CircuitBreaker as _, Config as CircuitBreaker, Error as Break};

impl RpcConnection {
    /// For a single RPC request, read the message, process it, then reply
    async fn handle_rpc(send: SendStream, recv: RecvStream, state: ServerState) -> Result<(), Error> {
        use rkyv::result::ArchivedResult;

        let mut stream = ::rpc::stream::RpcRecvReader::new(recv);

        match stream.recv::<Result<::rpc::request::RpcRequest, ApiError>>().await? {
            Some(ArchivedResult::Ok(msg)) => return self::rpc::dispatch(state, send, msg).await,
            Some(ArchivedResult::Err(e)) => log::warn!("Received error from gateway via RPC: {:?}", e.code()),
            None => log::warn!("Empty message from gateway"),
        }

        Ok(())
    }

    /// Listen for incoming bidirectional streams and treat them as RPC messages
    pub async fn run_rpc(self, state: ServerState) {
        let cb = CircuitBreaker::new().build();
        let mut tries = 0;

        loop {
            #[rustfmt::skip]
            let (send, recv) = match cb.call(self.conn.accept_bi()).await {
                Ok(stream) => {
                    tries = 0;
                    stream
                }
                Err(Break::Inner(ConnectionError::LocallyClosed | ConnectionError::ApplicationClosed(_) | ConnectionError::ConnectionClosed(_))) => {
                    log::error!("RPC Connection closed");
                    return; // connection is closed, end task
                }
                Err(e) => {
                    log::error!("RPC Connection error: {e}");
                    tries += 1;

                    if tries > 10 {
                        self.conn.close(VarInt::from_u32(405), b"Could Not Accept RPC Stream");
                    } else if matches!(e, Break::Rejected) {
                        // wait a second in case of something overloading
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }

                    continue; // try again to accept a connection
                }
            };

            let state = state.clone();
            tokio::spawn(async move {
                if let Err(e) = Self::handle_rpc(send, recv, state).await {
                    // TODO: Add to metrics
                    log::error!("Error handling RPC request: {e}");
                }
            });
        }
    }
}

impl GatewayConnection {
    /// While the connection is open, connect a unidirectional stream back to the gateway and send
    /// gateway events along it.
    pub async fn run_gateway(self, state: ServerState) {
        let cb = CircuitBreaker::new().build();
        let mut tries = 0;

        'connect: loop {
            let stream = match cb.call(self.conn.open_uni()).await {
                Ok(stream) => {
                    tries = 0;
                    stream
                }
                #[rustfmt::skip]
                Err(Break::Inner(ConnectionError::LocallyClosed | ConnectionError::ApplicationClosed(_) | ConnectionError::ConnectionClosed(_))) => {
                    log::error!("Gateway Connection closed");
                    return; // connection is closed, end task
                }
                Err(e) => {
                    log::error!("Cannot open Unidirectional gateway stream: {e}");

                    tries += 1;
                    if tries > 10 {
                        self.conn.close(VarInt::from_u32(404), b"Could Not Open");
                    } else if matches!(e, Break::Rejected) {
                        // wait a second in case of something overloading
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }

                    continue;
                }
            };

            let mut stream = AsyncFramedWriter::new(stream);
            let mut closed = std::pin::pin!(self.conn.closed());

            // this is organized such that upon reconnecting the connection will immediately send
            // any delayed or buffered events without having to wait for a new event to start the loop
            'recv: loop {
                'batch: loop {
                    // we can't access the event btree for long, so quickly collect events before streaming
                    let batched_events = state.gateway.events.batch_since(self.last_event.load(Ordering::Relaxed));

                    if batched_events.is_empty() {
                        break 'batch;
                    }

                    for (cnt, event) in batched_events {
                        if let Err(e) = stream.write_msg(event.as_slice()).await {
                            log::error!("Error writing event to gateway stream: {e}");
                            break 'recv;
                        }

                        self.last_event.store(cnt, Ordering::Relaxed);
                    }
                }

                tokio::select! { // wait for any new events or for the stream to close
                    _ = &mut closed => return, // if the connection closes, we're done here
                    _ = state.gateway.events.notify.notified() => continue 'recv,
                }
            }
        }
    }
}
