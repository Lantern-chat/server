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

use models::RoomPermissions;
use tokio::sync::{broadcast::error::RecvError, mpsc};
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream, ReceiverStream};

use hashbrown::HashMap;

use ftl::ws::{Message as WsMessage, SinkError, WebSocket};
use schema::Snowflake;
use util::cancel::{Cancel, CancelableStream};

use crate::{ctrl::auth::Authorization, permission_cache::PermMute, web::encoding::Encoding, ServerState};

use super::{
    conn::GatewayConnection,
    event::{EncodedEvent, Event},
    msg::{ClientMsg, ServerMsg},
};

pub mod params;
pub use params::GatewayQueryParams;

pub mod identify;

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error(transparent)]
    Broadcast(#[from] RecvError),

    #[error(transparent)]
    Oneshot(#[from] tokio::sync::oneshot::error::RecvError),
}

impl From<BroadcastStreamRecvError> for EventError {
    fn from(e: BroadcastStreamRecvError) -> Self {
        EventError::Broadcast(match e {
            BroadcastStreamRecvError::Lagged(l) => RecvError::Lagged(l),
        })
    }
}

pub enum Item {
    Event(Result<Event, EventError>),
    Msg(Result<ClientMsg, MessageIncomingError>),
    MissedHeartbeat,
}

lazy_static::lazy_static! {
    pub static ref HELLO_EVENT: Event = Event::new(ServerMsg::new_hello(models::events::Hello::default()), None).unwrap();
    pub static ref HEARTBEAT_ACK: Event = Event::new(ServerMsg::new_heartbeatack(), None).unwrap();
    pub static ref INVALID_SESSION: Event = Event::new(ServerMsg::new_invalidsession(), None).unwrap();
}

pub fn client_connected(ws: WebSocket, query: GatewayQueryParams, _addr: IpAddr, state: ServerState) {
    tokio::spawn(async move {
        let (conn, conn_rx) = GatewayConnection::new();
        let conn_rx = ReceiverStream::new(conn_rx);

        let (ws_tx, ws_rx) = ws.split();

        // map each incoming websocket message such that it will decompress/decode the message
        // AND update the last_msg value concurrently.
        let conn2 = conn.clone();
        let ws_rx = ws_rx
            .map(|msg| (msg, &conn2))
            .then(move |(msg, conn)| async move {
                match msg {
                    Err(e) => Err(MessageIncomingError::from(e)),
                    Ok(msg) if msg.is_close() => Err(MessageIncomingError::SocketClosed),
                    Ok(msg) => {
                        // Block to decompress and parse
                        let block =
                            tokio::task::spawn_blocking(move || -> Result<_, MessageIncomingError> {
                                let msg = decompress_if(query.compress, msg.as_bytes())?;

                                Ok(match query.encoding {
                                    Encoding::Json => serde_json::from_slice(&msg)?,
                                    Encoding::MsgPack => rmp_serde::from_slice(&msg)?,
                                })
                            });

                        // do the parsing/decompressing at the same time as updating the heartbeat
                        // TODO: Only count heartbeats on the actual event?
                        let (res, _) = future::join(block, conn.heartbeat()).await;

                        res?
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

        let mut listener_table = HashMap::new();

        let mut events = stream::SelectAll::<stream::BoxStream<Item>>::new();

        // Push Hello event to begin stream and forward ws_rx/conn_rx into events
        events.push(stream::once(future::ready(Item::Event(Ok(HELLO_EVENT.clone())))).boxed());
        events.push(ws_rx.map(|msg| Item::Msg(msg)).boxed());
        events.push(conn_rx.map(|msg| Item::Event(Ok(msg))).boxed());

        //let _ = conn.tx.send(HELLO_EVENT.clone()).await;

        // Make the new connection known to the gateway
        state.gateway.add_connection(conn.clone()).await;

        let mut user_id = None;
        let mut intent = models::Intent::empty();

        'event_loop: while let Some(event) = events.next().await {
            let resp = match event {
                Item::MissedHeartbeat => Err(MessageOutgoingError::SocketClosed),
                Item::Event(event) => match event {
                    Ok(event) => {
                        use super::msg::server::payloads::*;

                        // if this message corresponds to an intent, filter it
                        if let Some(matching_intent) = event.msg.matching_intent() {
                            if !intent.contains(matching_intent) {
                                continue; // skip doing anything with this event
                            }
                        }

                        match event.msg {
                            ServerMsg::Hello { .. } => {}
                            // TODO: Make this non-blocking for the event-loop?
                            ServerMsg::Ready {
                                payload: ReadyPayload { inner: ref ready },
                                ..
                            } => {
                                user_id = Some(ready.user.id);

                                // subscribe to all relevant party broadcasts
                                // and activate the connection for per-user events
                                let subs = state
                                    .gateway
                                    .sub_and_activate_connection(
                                        ready.user.id,
                                        conn.clone(),
                                        // NOTE: https://github.com/rust-lang/rust/issues/70263
                                        ready
                                            .parties
                                            .iter()
                                            .map(crate::util::passthrough(|p: &models::Party| &p.id)),
                                    )
                                    .await;

                                events.extend(subs.into_iter().map(|sub| {
                                    let (stream, cancel) =
                                        CancelableStream::new(BroadcastStream::new(sub.rx));

                                    listener_table.insert(sub.party_id, cancel);

                                    stream.map(|event| Item::Event(event.map_err(Into::into))).boxed()
                                }));
                            }
                            _ => match user_id {
                                None => break 'event_loop,
                                Some(user_id) => {
                                    if let Some(room_id) = event.room_id {
                                        match state.perm_cache.get(user_id, room_id).await {
                                            None => break 'event_loop,
                                            Some(PermMute { perm, .. }) => {
                                                if !perm.room.contains(RoomPermissions::READ_MESSAGES) {
                                                    continue 'event_loop;
                                                }
                                            }
                                        }
                                    }

                                    match event.msg {
                                        // TODO: Party ADD subscribe
                                        // TODO: Party REMOVE unsubscribe
                                        _ => {}
                                    }
                                }
                            },
                        }

                        Ok(event) // forward event directly to tx
                    }
                    Err(e) => {
                        log::warn!("Event error: {}", e);
                        Err(MessageOutgoingError::SocketClosed) // kick for lag
                    }
                },
                Item::Msg(msg) => match msg {
                    Ok(msg) => match msg {
                        // Respond to heartbeats immediately.
                        ClientMsg::Heartbeat { .. } => Ok(HEARTBEAT_ACK.clone()),
                        ClientMsg::Identify { payload, .. } => {
                            // this will send a ready event on success
                            tokio::spawn(identify::identify(
                                state.clone(),
                                conn.clone(),
                                payload.inner.auth,
                                payload.inner.intent,
                            ));
                            intent = payload.inner.intent;
                            continue;
                        }
                        ClientMsg::SetPresence { payload, .. } => {
                            // TODO;
                            continue;
                        }
                        // no immediate response necessary, continue listening for events
                        _ => continue,
                    },
                    Err(e) => match e {
                        _ if e.is_close() => {
                            log::warn!("Connection disconnected");
                            break;
                        }
                        // TODO: Send code with it
                        _ => {
                            log::error!("Misc err: {}", e);
                            Err(MessageOutgoingError::SocketClosed)
                        }
                    },
                },
            };

            // group together
            let flush_and_send = async {
                ws_tx.flush().await?;
                ws_tx.send(resp).await
            };

            match tokio::time::timeout(Duration::from_millis(45000), flush_and_send).await {
                Ok(Ok(())) => {
                    println!("Message sent!");
                }
                Ok(Err(e)) => {
                    todo!("Handle errors from websocket: {}", e);
                }
                Err(_timeout_error) => {
                    todo!("Force kick socket?")
                }
            }
        }

        // TODO: Cleanup connection
        // still missing per-user connection cleanup
        tokio::join! {
            state.gateway.conns.remove(&conn.id),
            async {
                if let Some(user_id) = user_id {
                    state.perm_cache.remove_reference(user_id).await;
                }
            },
        };
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

impl MessageIncomingError {
    pub fn is_close(&self) -> bool {
        use tokio_tungstenite::tungstenite::error::{Error, ProtocolError};

        match self {
            Self::SocketClosed => true,
            Self::TungsteniteError(e) => match e {
                Error::AlreadyClosed
                | Error::ConnectionClosed
                | Error::Protocol(ProtocolError::ResetWithoutClosingHandshake) => true,
                _ => false,
            },
            _ => false,
        }
    }
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
