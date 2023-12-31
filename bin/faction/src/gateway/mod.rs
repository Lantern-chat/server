#![allow(unused_labels)]

use std::sync::atomic::{AtomicU64, Ordering};

use crate::prelude::*;

pub type ConnectionId = Snowflake;

use tokio::{io::AsyncReadExt, sync::Notify};
use triomphe::Arc;

use framed::tokio::{AsyncFramedReader, AsyncFramedWriter};
use rkyv::{
    ser::{serializers::AllocSerializer, Serializer},
    AlignedVec, Archive, Serialize,
};

use quinn::{Connection, ConnectionError, RecvStream, SendStream, VarInt};

pub mod rpc;

const EVENT_BATCH_SIZE: usize = 64;

struct GatewayConnectionInner {
    pub id: ConnectionId,
    pub conn: Connection,
    pub last_event: AtomicU64,
    pub notify: Arc<Notify>,
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

pub struct Gateway {
    pub counter: AtomicU64,
    pub notify: Arc<Notify>,
    pub events: scc::TreeIndex<u64, Arc<AlignedVec>>,
    pub conns: scc::HashMap<ConnectionId, GatewayConnection>,
}

impl Gateway {
    pub fn new() -> Self {
        Gateway {
            counter: AtomicU64::new(0),
            notify: Arc::default(),
            events: Default::default(),
            conns: Default::default(),
        }
    }

    pub async fn insert_connection(&self, state: ServerState, conn: Connection) {
        let conn = GatewayConnection(Arc::new(GatewayConnectionInner {
            id: state.sf.gen(),
            conn,
            last_event: AtomicU64::new(0),
            notify: self.notify.clone(),
        }));

        tokio::spawn(conn.clone().run_rpc(state.clone()));
        tokio::spawn(conn.clone().run_gateway(state));

        self.conns.insert_async(conn.id, conn).await;
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

        let archived = serializer.into_serializer().into_inner();

        // TODO: Compression?
        self.events.insert_async(self.counter.fetch_add(1, Ordering::SeqCst), Arc::new(archived)).await;
        self.notify.notify_waiters();
    }
}

use failsafe::{futures::CircuitBreaker as _, Config as CircuitBreaker, Error as Break};

impl GatewayConnection {
    /// For a single RPC request, read the message, process it, then reply
    async fn handle_rpc(send: SendStream, recv: RecvStream, state: ServerState) -> Result<(), Error> {
        let mut recv = AsyncFramedReader::new(recv);

        let Some(msg) = recv.next_msg().await? else {
            return Err(Error::NotFound);
        };

        let mut buffer = AlignedVec::new();
        buffer.resize(msg.len() as usize, 0);
        msg.read_exact(&mut buffer[..]).await?;

        let msg = rkyv::check_archived_root::<::rpc::msg::Message>(&buffer).map_err(|e| {
            log::error!("Error getting archived RPC message: {e}");
            Error::RkyvEncodingError
        })?;

        self::rpc::dispatch(state, AsyncFramedWriter::new(send), msg).await
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
                        self.conn.close(VarInt::from_u32(405), b"Could Not Accept Stream");
                    } else if matches!(e, Break::Rejected) {
                        // wait a second in case of something overloading
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }

                    continue; // try again to accept a connection
                }
            };

            let state = state.clone();
            tokio::spawn(async move {
                if let Err(e) = GatewayConnection::handle_rpc(send, recv, state).await {
                    // TODO: Add to metrics
                    log::error!("Error handling RPC request: {e}");
                }
            });
        }
    }

    /// While the connection is open, connect a unidirectional stream back to the gateway and send
    /// gateway events along it.
    pub async fn run_gateway(self, state: ServerState) {
        let cb = CircuitBreaker::new().build();
        let mut tries = 0;

        'connect: loop {
            #[rustfmt::skip]
            let stream = match cb.call(self.conn.open_uni()).await {
                Ok(stream) => {
                    tries = 0;
                    stream
                }
                Err(Break::Inner(ConnectionError::LocallyClosed | ConnectionError::ApplicationClosed(_) | ConnectionError::ConnectionClosed(_))) => {
                    log::error!("RPC Connection closed");
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
                    #[rustfmt::skip]
                    let batched_events: Vec<_> = state.gateway.events
                        .range(self.last_event.load(Ordering::Relaxed).., &scc::ebr::Guard::new())
                        .take(EVENT_BATCH_SIZE).map(|(&k, v)| (k, v.clone())).collect();

                    if batched_events.is_empty() {
                        break 'batch;
                    }

                    for (cnt, event) in batched_events {
                        if let Err(e) = stream.write_msg(event.as_slice()).await {
                            log::error!("Error writing event to uni gateway stream: {e}");
                            break 'recv;
                        }

                        self.last_event.store(cnt, Ordering::Relaxed);
                    }
                }

                tokio::select! { // wait for any new events or for the stream to close
                    _ = &mut closed => return, // if the connection closes, we're done here
                    _ = self.notify.notified() => continue 'recv,
                }
            }
        }
    }
}
