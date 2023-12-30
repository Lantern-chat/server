use crate::prelude::*;

pub type ConnectionId = Snowflake;

use tokio::{
    io::AsyncReadExt,
    sync::broadcast::{self, error::RecvError},
};
use triomphe::Arc;

use framed::tokio::{AsyncFramedReader, AsyncFramedWriter};
use rkyv::{
    ser::{serializers::AllocSerializer, Serializer},
    AlignedVec, Archive, Serialize,
};

use quinn::{Connection, ConnectionError, RecvStream, SendStream, VarInt};

pub mod rpc;

#[derive(Clone)]
pub struct GatewayConnection {
    pub id: ConnectionId,
    pub conn: Connection,
}

pub struct Gateway {
    /// All gateway connections will listen to this in their own task and forward the bytes
    /// along their own unidirectional stream for the gateway server to receive.
    pub tx: broadcast::Sender<Arc<AlignedVec>>,
    pub conns: scc::HashIndex<ConnectionId, GatewayConnection>,
}

impl Gateway {
    pub async fn insert_connection(&self, state: ServerState, conn: Connection) -> ConnectionId {
        let id = state.sf.gen();
        let conn = GatewayConnection { id, conn };

        tokio::spawn(conn.clone().run_rpc(state));
        tokio::spawn(conn.clone().run_gateway(self.tx.subscribe()));

        self.conns.insert_async(id, conn).await;

        id
    }

    pub async fn send<T, const N: usize>(&self, event: &T) -> Result<(), Error>
    where
        T: Archive + Serialize<AllocSerializer<N>>,
    {
        let mut serializer = AllocSerializer::<N>::default();

        if let Err(e) = serializer.serialize_value(event) {
            log::error!("Rkyv Error: {e}");
            return Err(Error::RkyvEncodingError);
        }

        let archived = serializer.into_serializer().into_inner();
        if let Err(e) = self.tx.send(Arc::new(archived)) {
            //crate::metrics::API_METRICS.load().errs.add(1);
            log::error!("Could not broadcast event: {e}");
        }

        Ok(())
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

        let msg = rkyv::check_archived_root::<rpc::msg::Message>(&buffer).map_err(|e| {
            log::error!("Error getting archived RPC message: {e}");
            Error::RkyvEncodingError
        })?;

        rpc::dispatch(state, AsyncFramedWriter::new(send), msg).await
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
                    log::error!("Error handling RPC request: {e}");
                }
            });
        }
    }

    /// While the connection is open, connect a unidirectional stream back to the gateway and send
    /// gateway events along it.
    pub async fn run_gateway(self, mut rx: broadcast::Receiver<Arc<AlignedVec>>) {
        let cb = CircuitBreaker::new().build();
        let mut tries = 0;

        loop {
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

            'recv: loop {
                let event = tokio::select! {
                    _ = &mut closed => return, // if the connection closes, we're done here
                    event = rx.recv() => match event {
                        Ok(event) => event,
                        Err(RecvError::Closed) => return, // also done if the event stream ends
                        Err(_) => {
                            self.conn.close(VarInt::from_u32(200), b"Event stream lagged");
                            break 'recv;
                        }
                    }
                };

                if let Err(e) = stream.write_msg(event.as_slice()).await {
                    log::error!("Error writing event to uni gateway stream: {e}");
                    break 'recv;
                }
            }
        }
    }
}
