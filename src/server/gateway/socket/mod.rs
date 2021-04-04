use std::{
    borrow::Cow,
    error::Error,
    net::IpAddr,
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};

use futures::{
    future::{self, Either},
    stream, Future, FutureExt, SinkExt, Stream, StreamExt, TryStreamExt,
};

use tokio::sync::{broadcast::error::RecvError, mpsc};
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
        static ref HELLO_EVENT: Event = Event::new_opaque(ServerMsg::new_hello(45000)).unwrap();
        static ref HEARTBEAT_ACK: Event = Event::new_opaque(ServerMsg::new_heartbeatack()).unwrap();
    }

    let (ws_tx, ws_rx) = ws.split();

    let ws_rx = ws_rx.then(move |msg| async move {
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
    let mut ws_tx = ws_tx.with(move |event: Result<Event, MessageOutgoingError>| {
        futures::future::ok::<_, SinkError>(match event {
            Err(_) => WsMessage::close(),
            Ok(event) => WsMessage::binary(event.encoded.get(query).clone()),
        })
    });

    tokio::spawn(async move {
        type LeftItem = Result<Event, RecvError>;
        type RightItem = Result<ClientMsg, MessageIncomingError>;
        type Item = Either<LeftItem, RightItem>;

        let mut events = stream::SelectAll::<stream::BoxStream<Item>>::new();

        // Push Hello event to begin stream and forward ws_rx into events
        events.push(stream::once(future::ready(Either::Left(Ok(HELLO_EVENT.clone())))).boxed());
        events.push(ws_rx.map(|msg| Either::Right(msg)).boxed());

        while let Some(event) = events.next().await {
            let resp = match event {
                Either::Left(event) => match event {
                    Ok(event) => {
                        // TODO: Check event for updates
                        Ok(event)
                    }
                    Err(e) => {
                        log::warn!("Event error: {}", e);
                        Err(MessageOutgoingError::SocketClosed) // kick
                    }
                },
                Either::Right(msg) => match msg {
                    Ok(msg) => {
                        todo!("Handle message");
                        continue; // no immediate response necessary, continue listening for events
                    }
                    Err(e) => match e {
                        MessageIncomingError::SocketClosed
                        | MessageIncomingError::TungsteniteError(SinkError::ConnectionClosed)
                        | MessageIncomingError::TungsteniteError(SinkError::AlreadyClosed) => {
                            log::warn!("Connection disconnected");
                            break;
                        }
                        MessageIncomingError::JsonParseError(_)
                        | MessageIncomingError::MsgParseError(_)
                        | MessageIncomingError::IoError(_) => {
                            // TODO: Send code with it
                            Err(MessageOutgoingError::SocketClosed)
                        }
                        _ => Err(MessageOutgoingError::SocketClosed),
                    },
                },
            };

            let flush_and_send = async {
                ws_tx.flush().await?;
                ws_tx.send(resp).await
            };

            match tokio::time::timeout(Duration::from_millis(45000), flush_and_send).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    todo!("Handle errors from websocket")
                }
                Err(timeout_error) => {
                    todo!("Force kick socket?")
                }
            }
        }

        // TODO: Cleanup connection

        /*
        match ws_tx.send(Ok(HELLO_EVENT.clone())).await {
            Ok(_) => {}
            Err(SinkError::ConnectionClosed) | Err(SinkError::AlreadyClosed) => {
                log::warn!("Connection disconnected before Hello could be sent");

                return;
            }
            Err(e) => {
                log::error!("Error sending Hello message: {}", e);

                if let Err(e) = ws_tx.send(Err(MessageOutgoingError::SocketClosed)).await {
                    log::error!("Error sending close message: {}", e);
                }
            }
        }
        */
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
