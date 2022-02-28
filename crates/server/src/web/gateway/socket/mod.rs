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
    stream::{self, AbortHandle, Abortable, BoxStream, SelectAll},
    Future, FutureExt, SinkExt, Stream, StreamExt, TryStreamExt,
};

use sdk::models::RoomPermissions;
use tokio::sync::{broadcast::error::RecvError, mpsc};
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream, ReceiverStream};

use hashbrown::HashMap;

use ftl::ws::{Message as WsMessage, SinkError, WebSocket};
use schema::Snowflake;

use crate::{
    ctrl::{
        auth::Authorization,
        gateway::presence::{clear_presence, set_presence},
    },
    permission_cache::PermMute,
    web::encoding::Encoding,
    ServerState,
};

use super::{
    conn::GatewayConnection,
    event::{EncodedEvent, Event},
    PartySubscription,
};

use sdk::models::gateway::message::{ClientMsg, ServerMsg};

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
    pub static ref HELLO_EVENT: Event = Event::new(ServerMsg::new_hello(sdk::models::events::Hello::default()), None).unwrap();
    pub static ref HEARTBEAT_ACK: Event = Event::new(ServerMsg::new_heartbeat_ack(), None).unwrap();
    pub static ref INVALID_SESSION: Event = Event::new(ServerMsg::new_invalid_session(), None).unwrap();
}

type ListenerTable = HashMap<Snowflake, AbortHandle>;

pub fn client_connected(ws: WebSocket, query: GatewayQueryParams, _addr: IpAddr, state: ServerState) {
    tokio::spawn(async move {
        let (conn, conn_rx) = GatewayConnection::new();
        let conn_rx = ReceiverStream::new(conn_rx);

        let (ws_tx, ws_rx) = ws.split();

        // map each incoming websocket message such that it will decompress/decode the message
        // AND update the last_msg value concurrently.
        let conn2 = conn.clone();
        let ws_rx = ws_rx.map(|msg| (msg, &conn2)).then(move |(msg, conn)| async move {
            match msg {
                Err(e) => Err(MessageIncomingError::from(e)),
                Ok(msg) if msg.is_close() => Err(MessageIncomingError::SocketClosed),
                Ok(msg) => {
                    // Block to decompress and parse
                    let block = tokio::task::spawn_blocking(move || -> Result<_, MessageIncomingError> {
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

        // for each party that is being listened on, keep the associated cancel handle, to kill the stream if we unsub from them
        let mut listener_table = HashMap::new();

        // aggregates all event streams into one
        let mut events: SelectAll<BoxStream<Item>> = SelectAll::<BoxStream<Item>>::new();

        // Push Hello event to begin stream and forward ws_rx/conn_rx into events
        events.push(stream::once(future::ready(Item::Event(Ok(HELLO_EVENT.clone())))).boxed());
        events.push(ws_rx.map(|msg| Item::Msg(msg)).boxed());
        events.push(conn_rx.map(|msg| Item::Event(Ok(msg))).boxed());

        // Make the new connection known to the gateway
        state.gateway.add_connection(conn.clone()).await;

        let mut user_id = None;
        let mut intent = sdk::models::Intent::empty();

        'event_loop: while let Some(event) = events.next().await {
            let resp = match event {
                Item::MissedHeartbeat => Err(MessageOutgoingError::SocketClosed),

                Item::Event(event) => match event {
                    Ok(event) => {
                        use sdk::models::gateway::message::server_msg_payloads::*;

                        // if this message corresponds to an intent, filter it
                        if let Some(matching_intent) = event.msg.matching_intent() {
                            if !intent.contains(matching_intent) {
                                continue; // skip doing anything with this event
                            }
                        }

                        match event.msg {
                            ServerMsg::Hello { .. } => {}
                            ServerMsg::InvalidSession { .. } => {
                                // this will ensure the stream ends after this event
                                events.clear();
                            }
                            ServerMsg::Ready {
                                payload: ReadyPayload { inner: ref ready },
                                ..
                            } => {
                                user_id = Some(ready.user.id);

                                register_subs(
                                    &mut events,
                                    &mut listener_table,
                                    state
                                        .gateway
                                        .sub_and_activate_connection(
                                            ready.user.id,
                                            conn.clone(),
                                            // NOTE: https://github.com/rust-lang/rust/issues/70263
                                            ready.parties.iter().map(crate::util::passthrough(|p: &sdk::models::Party| &p.id)),
                                        )
                                        .await,
                                )
                            }
                            // for other events, session must be authenticated and have permission to view such events
                            _ => match user_id {
                                None => {
                                    log::warn!("Attempted to receive events before user_id was set");
                                    break 'event_loop;
                                }
                                Some(user_id) => {
                                    if let Some(room_id) = event.room_id {
                                        match state.perm_cache.get(user_id, room_id).await {
                                            None => {
                                                // if no permission cache was found, refresh it.
                                                // takes ownership of the event and will retry when done
                                                tokio::spawn(refresh_and_retry(state.clone(), conn.clone(), event, user_id, room_id));
                                                continue 'event_loop;
                                            }
                                            // skip event if user can't view room
                                            Some(perms) if !perms.room.contains(RoomPermissions::VIEW_ROOM) => continue 'event_loop,
                                            _ => { /* send message as normal*/ }
                                        }
                                    }

                                    match event.msg {
                                        ServerMsg::PartyCreate { ref payload, .. } => register_subs(
                                            &mut events,
                                            &mut listener_table,
                                            state.gateway.sub_and_activate_connection(user_id, conn.clone(), &[payload.inner.id]).await,
                                        ),
                                        ServerMsg::PartyDelete { ref payload, .. } => {
                                            // by cancelling a stream, it will be removed from the SelectStream automatically
                                            if let Some(event_stream) = listener_table.get(&payload.id) {
                                                event_stream.abort();
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            },
                        }

                        Ok(event) // forward event directly to tx
                    }
                    Err(e) => {
                        log::warn!("Event error: {e}");
                        Err(MessageOutgoingError::SocketClosed) // kick for lag?
                    }
                },
                Item::Msg(msg) => match msg {
                    Ok(msg) => match msg {
                        // Respond to heartbeats immediately.
                        ClientMsg::Heartbeat { .. } => Ok(HEARTBEAT_ACK.clone()),
                        ClientMsg::Identify { payload, .. } => {
                            // this will send a ready event on success
                            tokio::spawn(identify::identify(state.clone(), conn.clone(), payload.inner.auth, payload.inner.intent));
                            intent = payload.inner.intent;
                            continue;
                        }
                        ClientMsg::Resume { .. } => {
                            log::error!("Attempted to resume connection");
                            break 'event_loop;
                        }
                        ClientMsg::SetPresence { payload, .. } => {
                            match user_id {
                                None => {
                                    log::warn!("Attempted to set presence before identification");
                                    break 'event_loop;
                                }
                                Some(user_id) => {
                                    tokio::spawn(set_presence(state.clone(), user_id, conn.id, payload.inner.presence));
                                }
                            }

                            continue 'event_loop; // no reply, so continue event loop
                        }
                        ClientMsg::Subscribe { .. } | ClientMsg::Unsubscribe { .. } => {
                            log::error!("Unimplemented sub/unsub");
                            continue 'event_loop; // no reply
                        }
                    },
                    Err(e) => match e {
                        _ if e.is_close() => {
                            log::warn!("Connection disconnected");
                            break;
                        }
                        // TODO: Send code with it
                        _ => {
                            log::error!("Misc err: {e}");
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
                    log::error!("Handle errors from websocket: {e}");
                    break 'event_loop;
                }
                Err(_timeout_error) => {
                    log::error!("Force kick socket?");
                    break 'event_loop;
                }
            }
        } // END 'event_loop

        log::trace!("Gateway event loop ended");

        // un-cache permissions
        if let Some(user_id) = user_id {
            state.perm_cache.remove_reference(user_id).await;

            // if there was a user_id, that means the connection had been readied and a presence possibly set,
            // so kick off a task that will clear the presence after 5 seconds.
            //
            // 5 seconds would give enough time for a page reload, so if the user starts a new connection before then
            // we can avoid flickering presences
            let conn_id = conn.id;
            let state2 = state.clone();
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(5)).await;

                if let Err(e) = clear_presence(state2, conn_id).await {
                    log::error!("Error clearing connection presence: {e}");
                }
            });
        }

        // remove connection from gateway tables
        state.gateway.remove_connection(conn.id, user_id).await;
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
            Self::TungsteniteError(e) => matches!(
                e,
                Error::AlreadyClosed | Error::ConnectionClosed | Error::Protocol(ProtocolError::ResetWithoutClosingHandshake)
            ),
            _ => false,
        }
    }
}

#[inline]
fn decompress_if(cond: bool, msg: &[u8]) -> Result<Cow<[u8]>, std::io::Error> {
    if !cond {
        return Ok(Cow::Borrowed(msg));
    }

    use miniz_oxide::inflate::{self, TINFLStatus};

    let err = match inflate::decompress_to_vec_zlib(msg) {
        Ok(decompressed) => return Ok(Cow::Owned(decompressed)),
        Err(err) => match err {
            TINFLStatus::Done => unreachable!("TINFLStatus::Done"),
            TINFLStatus::FailedCannotMakeProgress => "Truncated Stream",
            TINFLStatus::BadParam => "Bad Param",
            TINFLStatus::Adler32Mismatch => "Adler32 Mismatch",
            _ => "Corrupt Stream",
        },
    };

    return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, err));
}

async fn refresh_and_retry(state: ServerState, conn: GatewayConnection, event: Event, user_id: Snowflake, room_id: Snowflake) {
    if let Ok(db) = state.db.read.get().await {
        if let Ok(_) = crate::ctrl::gateway::refresh::refresh_room_perms(&state, &db, user_id).await {
            // double-check once refreshed. Only if it really exists should it continue.
            if state.perm_cache.get(user_id, room_id).await.is_some() {
                // we don't care about the result of this
                let _ = conn.tx.send(event).await;

                return;
            }
        }
    }

    // if we *still* don't have the permissions or an error occured, kick.
    let _ = conn.tx.send(INVALID_SESSION.clone()).await;
}

// TODO: to implement sub/unsub low-bandwidth modes, the abort handles will
// probably need to be replaced with something that pauses them instead
fn register_subs(events: &mut SelectAll<BoxStream<Item>>, listener_table: &mut ListenerTable, subs: Vec<PartySubscription>) {
    // iterate through subscribed parties
    events.extend(subs.into_iter().map(|sub| {
        // take their broadcast stream and wrap it in an abort signal
        // this is so we can unsubscribe later if needed
        let (stream, handle) = stream::abortable(BroadcastStream::new(sub.rx));

        listener_table.insert(sub.party_id, handle);

        // map the stream to the `Item` type
        stream.map(|event| Item::Event(event.map_err(Into::into))).boxed()
    }));
}
