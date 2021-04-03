use std::{borrow::Cow, error::Error, net::IpAddr, pin::Pin, sync::Arc, time::Instant};

use futures::{future, Future, FutureExt, SinkExt, StreamExt, TryStreamExt};

use tokio::sync::mpsc;
use tokio_postgres::Socket;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{
    db::Snowflake,
    server::ftl::ws::{Message as WsMessage, SinkError, WebSocket},
    util::laggy,
};

use crate::server::{routes::api::auth::Authorization, ServerState};

use super::{
    event::{EncodedEvent, Event},
    msg::{ClientMsg, ServerMsg},
};

pub mod params;
pub use params::{GatewayMsgEncoding, GatewayQueryParams};

pub fn client_connected(
    ws: WebSocket,
    query: GatewayQueryParams,
    addr: IpAddr,
    state: ServerState,
) {
    lazy_static::lazy_static! {
        static ref HELLO_EVENT: EncodedEvent = EncodedEvent::new(&ServerMsg::new_hello(45000)).unwrap();
        static ref HEARTBEAT_ACK: EncodedEvent = EncodedEvent::new(&ServerMsg::new_heartbeatack()).unwrap();
    }

    let (ws_tx, ws_rx) = ws.split();

    let mut ws_rx = ws_rx.then(|msg| async move {
        match msg {
            Err(e) => Err(MessageIncomingError::from(e)),
            Ok(msg) if msg.is_close() => Err(MessageIncomingError::SocketClosed),
            Ok(msg) => {
                // Block to decompress and parse
                tokio::task::spawn_blocking(move || -> Result<_, MessageIncomingError> {
                    let msg = decompress_if(query.compress, msg.as_bytes())?;

                    Ok(match query.encoding {
                        GatewayMsgEncoding::Json => serde_json::from_slice(&msg)?,
                        GatewayMsgEncoding::MsgPack => rmp_serde::from_slice(&msg)?,
                    })
                })
                .await?
            }
        }
    });

    // by placing the WsMessage constructor here, it avoids allocation ahead of when it can send the message
    let mut ws_tx = ws_tx.with(|event: Result<Event, MessageOutgoingError>| {
        futures::future::ok::<_, SinkError>(match event {
            Err(_) => WsMessage::close(),
            Ok(event) => WsMessage::binary(event.encoded.get(query).clone()),
        })
    });

    pub enum SocketState {
        Hello,
        Realtime,
        Catchup,
    }

    tokio::spawn(async move {

        // TODO
    });
}

#[derive(Debug, thiserror::Error)]
pub enum MessageOutgoingError {
    #[error("Socket Closed")]
    SocketClosed,
}

#[derive(Debug, thiserror::Error)]
pub enum MessageIncomingError {
    #[error("Tungstentite Error: {0}")]
    TungsteniteError(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Socket Closed")]
    SocketClosed,

    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON Parse Error: {0}")]
    JsonParseError(#[from] serde_json::Error),

    #[error("MsgPack Parse Error: {0}")]
    MsgParseError(#[from] rmp_serde::decode::Error),

    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}

#[inline]
fn decompress_if(cond: bool, msg: &[u8]) -> Result<Cow<[u8]>, std::io::Error> {
    if !cond {
        return Ok(Cow::Borrowed(msg));
    }

    use flate2::bufread::ZlibDecoder;
    use std::io::Read;

    let mut reader = ZlibDecoder::new(&*msg);
    let mut decoded = Vec::with_capacity(128);

    reader.read_to_end(&mut decoded)?;

    Ok(Cow::Owned(decoded))
}
