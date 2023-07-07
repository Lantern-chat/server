use std::{borrow::Cow, net::IpAddr, time::Duration};

use futures::{
    future,
    stream::{self, AbortHandle, BoxStream, SelectAll},
    FutureExt, SinkExt, StreamExt,
};

use sdk::{api::gateway::GatewayQueryParams, driver::Encoding, models::Permissions};
use tokio::sync::broadcast::error::RecvError;
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream, ReceiverStream};

use hashbrown::{HashMap, HashSet};

use ftl::ws::{Message as WsMessage, SinkError, WebSocket};
use schema::Snowflake;

use crate::{
    backend::{
        api::gateway::presence::{clear_presence, set_presence},
        cache::permission_cache::PermMute,
        gateway::{
            conn::GatewayConnection,
            event::{Event, EventInner, InternalEvent},
            PartySubscription,
        },
    },
    ServerState,
};

use sdk::models::gateway::message::{ClientMsg, ServerMsg};

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
    Ping,
    MissedHeartbeat,
}

lazy_static::lazy_static! {
    pub static ref HELLO_EVENT: Event = Event::new_compressed(ServerMsg::new_hello(sdk::models::events::Hello::default()), None, 10).unwrap();
    pub static ref HEARTBEAT_ACK: Event = Event::new_compressed(ServerMsg::new_heartbeat_ack(), None, 10).unwrap();
    pub static ref INVALID_SESSION: Event = Event::new_compressed(ServerMsg::new_invalid_session(), None, 10).unwrap();
}

type ListenerTable = HashMap<Snowflake, AbortHandle>;

pub fn client_connected(ws: WebSocket, query: GatewayQueryParams, addr: IpAddr, state: ServerState) {
    tokio::spawn(client_connection(ws, query, addr, state));
}

pub async fn client_connection(ws: WebSocket, query: GatewayQueryParams, _addr: IpAddr, state: ServerState) {
    let (ws_tx, ws_rx) = ws.split();

    let (conn, conn_rx) = state.gateway.new_connection().await;
    let conn_rx = ReceiverStream::new(conn_rx);

    // map each incoming websocket message such that it will decompress/decode the message
    // AND update the last_msg value concurrently.
    let conn2 = conn.clone();
    let ws_rx = ws_rx.map(|msg| (msg, &conn2)).then(move |(msg, conn)| async move {
        match msg {
            Err(e) => Item::Msg(Err(MessageIncomingError::from(e))),
            Ok(msg) if msg.is_close() => Item::Msg(Err(MessageIncomingError::SocketClosed)),
            Ok(msg) if msg.is_ping() => {
                conn.heartbeat().await;

                Item::Ping
            }
            Ok(msg) => {
                // Block to decompress and parse
                let block = tokio::task::spawn_blocking(move || -> Result<_, MessageIncomingError> {
                    let msg = decompress_if(query.compress, msg.as_bytes())?;

                    Ok(match query.encoding {
                        Encoding::JSON => serde_json::from_slice(&msg)?,
                        Encoding::CBOR => ciborium::de::from_reader(&msg[..])?,
                    })
                });

                // do the parsing/decompressing at the same time as updating the heartbeat
                // TODO: Only count heartbeats on the actual event?
                let (res, _) = tokio::join!(block, conn.heartbeat());

                match res {
                    Ok(msg) => Item::Msg(msg),
                    Err(e) => Item::Msg(Err(e.into())),
                }
            }
        }
    });

    // by placing the WsMessage constructor here, it avoids allocation ahead of when it can send the message
    let mut ws_tx = ws_tx.with(move |event: Result<Event, MessageOutgoingError>| {
        futures::future::ok::<_, SinkError>(match event {
            Err(_) => WsMessage::close(),
            Ok(event) => match *event {
                EventInner::External(ref event) => WsMessage::binary(event.encoded.get(query).clone()),
                _ => unreachable!(),
            },
        })
    });

    // for each party that is being listened on, keep the associated cancel handle, to kill the stream if we unsub from them
    let mut listener_table = HashMap::new();

    // Contains a list of user ids that have blocked the current user of this connection
    let mut blocked_by: HashSet<Snowflake> = HashSet::default();
    let mut roles = RoleCache::default();

    // aggregates all event streams into one
    let mut events: SelectAll<BoxStream<Item>> = SelectAll::<BoxStream<Item>>::new();

    // Push Hello event to begin stream and forward ws_rx/conn_rx into events
    events.push(stream::once(future::ready(Item::Event(Ok(HELLO_EVENT.clone())))).boxed());
    events.push(ws_rx.boxed());
    events.push(conn_rx.map(|msg| Item::Event(Ok(msg))).boxed());

    let mut user_id = None;
    let mut intent = sdk::models::Intent::empty();

    'event_loop: while let Some(event) = events.next().await {
        let resp = match event {
            Item::MissedHeartbeat => Err(MessageOutgoingError::SocketClosed),

            // Pong resposes should be handled by the underlying socket,
            // but we still need to ignore the message
            Item::Ping => continue,

            Item::Event(Err(e)) => {
                log::warn!("Event error: {e}");
                Err(MessageOutgoingError::SocketClosed) // kick for lag?
            }

            Item::Event(Ok(mut event)) => {
                let e = match *event {
                    EventInner::External(ref e) => e,
                    EventInner::Internal(ref event) => {
                        match event {
                            InternalEvent::BulkUserBlockedRefresh { blocked } => {
                                blocked_by.clear();
                                blocked_by.extend(blocked);
                            }
                            InternalEvent::UserBlockedAdd { user_id } => {
                                blocked_by.insert(*user_id);
                            }
                            InternalEvent::UserBlockedRemove { user_id } => {
                                blocked_by.remove(user_id);
                            }
                        }

                        continue;
                    }
                };

                // if this message corresponds to an intent, filter it
                if let Some(matching_intent) = e.msg.matching_intent() {
                    if !intent.contains(matching_intent) {
                        continue; // skip doing anything with this event
                    }
                }

                if let Some(user_id) = e.msg.user_id() {
                    if blocked_by.contains(&user_id) {
                        // TODO: Replace message create with "message unavailable" create
                        continue; // skip sending this to a user that's been blocked
                    }
                }

                // Honestly these are just grouped together to reduce indentation
                match (user_id, &e.msg) {
                    (_, ServerMsg::Hello(_)) => {}
                    (_, ServerMsg::InvalidSession(_)) => {
                        // this will ensure the stream ends after this event
                        events.clear();
                    }
                    (_, ServerMsg::Ready(ref ready)) => {
                        use sdk::models::gateway::events::ReadyParty;

                        user_id = Some(ready.user.id);

                        for party in &ready.parties {
                            if let Some(ref my_roles) = party.me.roles {
                                roles.add(party.id, my_roles);
                            }
                        }

                        register_subs(
                            &mut events,
                            &mut listener_table,
                            state
                                .gateway
                                .sub_and_activate_connection(
                                    ready.user.id,
                                    conn.clone(),
                                    // NOTE: https://github.com/rust-lang/rust/issues/70263
                                    ready.parties.iter().map(crate::util::passthrough(|p: &ReadyParty| &p.id)),
                                )
                                .boxed()
                                .await,
                        )
                    }
                    (None, _) => {
                        log::warn!("Attempted to receive events before user_id was set");
                        break 'event_loop;
                    }
                    // for other events, session must be authenticated and have permission to view such events
                    (Some(user_id), _) => {
                        use sdk::models::gateway::message::server_msg_payloads::{MemberRemovePayload, MemberUpdatePayload};

                        // if the event indicates things that can invalidate the permission cache, they must be handled
                        let clear_user = match e.msg {
                            // role updates when the current user has this role
                            ServerMsg::RoleUpdate(ref r) if roles.has(r.party_id, r.id) => true,
                            ServerMsg::RoleDelete(ref r) if roles.has(r.party_id, r.id) => {
                                roles.remove_role(r.party_id, r.id);
                                true
                            }
                            // member events for the current user
                            ServerMsg::MemberUpdate(MemberUpdatePayload { ref inner }) | ServerMsg::MemberRemove(MemberRemovePayload { ref inner })
                                if inner.member.user.id == user_id =>
                            {
                                // remove old roles and add new
                                roles.remove_party(inner.party_id);
                                if let Some(ref role_ids) = inner.member.roles {
                                    roles.add(inner.party_id, role_ids);
                                }

                                true
                            }
                            ServerMsg::PartyDelete(ref p) => {
                                roles.remove_party(p.id);
                                true
                            }
                            ServerMsg::RoomUpdate(ref r) => {
                                // TODO
                                true
                            }
                            _ => false,
                        };

                        if clear_user {
                            state.perm_cache.clear_user(user_id).await;
                        }

                        let mut now_invalid = false;

                        if let Some(room_id) = e.room_id {
                            match state.perm_cache.get(user_id, room_id).await {
                                None => match refresh(state.clone(), user_id, room_id).boxed().await {
                                    Ok(Some(perms)) => {
                                        // skip this event if they don't have permissions to view it
                                        if !perms.contains(Permissions::VIEW_ROOM) {
                                            continue 'event_loop;
                                        }
                                    }
                                    Ok(None) => {
                                        now_invalid = true;
                                    }
                                    Err(e) => {
                                        log::error!("Error refreshing user {user_id} room {room_id} permissions: {e}");
                                        break 'event_loop;
                                    }
                                },
                                // skip event if user can't view room
                                Some(perms) if !perms.contains(Permissions::VIEW_ROOM) => continue 'event_loop,
                                _ => { /* send message as normal*/ }
                            }
                        }

                        match e.msg {
                            _ if now_invalid => {
                                event = INVALID_SESSION.clone();
                                events.clear();
                            }
                            ServerMsg::PartyCreate(ref payload) => register_subs(
                                &mut events,
                                &mut listener_table,
                                state.gateway.sub_and_activate_connection(user_id, conn.clone(), &[payload.id]).boxed().await,
                            ),
                            ServerMsg::PartyDelete(ref payload) => {
                                // by cancelling a stream, it will be removed from the SelectStream automatically
                                if let Some(event_stream) = listener_table.get(&payload.id) {
                                    event_stream.abort();
                                }
                            }
                            _ => {}
                        }
                    }
                }

                Ok(event) // forward event directly to tx
            }
            Item::Msg(Err(e)) => match e {
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
            Item::Msg(Ok(msg)) => match msg {
                // Respond to heartbeats immediately.
                ClientMsg::Heartbeat(_) => Ok(HEARTBEAT_ACK.clone()),
                ClientMsg::Identify(payload) => {
                    // this will send a ready event on success
                    tokio::spawn(identify::identify(state.clone(), conn.clone(), payload.inner.auth, payload.intent));
                    intent = payload.intent;
                    continue;
                }
                ClientMsg::Resume(_) => {
                    log::error!("Attempted to resume connection");
                    break 'event_loop;
                }
                ClientMsg::SetPresence(payload) => {
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
                ClientMsg::Subscribe(_) | ClientMsg::Unsubscribe(_) => {
                    log::error!("Unimplemented sub/unsub");
                    continue 'event_loop; // no reply
                }
            },
        };

        // group together
        let flush_and_send = async {
            ws_tx.flush().await?;
            ws_tx.send(resp).await
        };

        match tokio::time::timeout(Duration::from_millis(45000), flush_and_send).await {
            Ok(Ok(())) => {}
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
        let state = state.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;

            if let Err(e) = clear_presence(state, user_id, conn_id).await {
                log::error!("Error clearing connection presence: {e}");
            }
        });
    }

    // remove connection from gateway tables
    state.gateway.remove_connection(conn.id, user_id).await;
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

    #[error("Cbor Parse Error: {0}")]
    CborParseError(#[from] ciborium::de::Error<std::io::Error>),

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
        Err(err) => match err.status {
            TINFLStatus::Done => unreachable!("TINFLStatus::Done"),
            TINFLStatus::FailedCannotMakeProgress => "Truncated Stream",
            TINFLStatus::BadParam => "Bad Param",
            TINFLStatus::Adler32Mismatch => "Adler32 Mismatch",
            _ => "Corrupt Stream",
        },
    };

    Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, err))
}

async fn refresh(state: ServerState, user_id: Snowflake, room_id: Snowflake) -> Result<Option<PermMute>, crate::Error> {
    let db = state.db.read.get().await?;

    crate::backend::api::gateway::refresh::refresh_room_perms(&state, &db, user_id).await?;

    Ok(state.perm_cache.get(user_id, room_id).await)
}

// async fn refresh_and_retry(state: ServerState, conn: GatewayConnection, event: Event, user_id: Snowflake, room_id: Snowflake) {
//     if let Ok(db) = state.db.read.get().await {
//         if let Ok(_) = crate::backend::api::gateway::refresh::refresh_room_perms(&state, &db, user_id).await {
//             // double-check once refreshed. Only if it really exists should it continue.
//             if state.perm_cache.get(user_id, room_id).await.is_some() {
//                 // we don't care about the result of this
//                 let _ = conn.tx.send(event).await;

//                 return;
//             }
//         }
//     }

//     // if we *still* don't have the permissions or an error occured, kick.
//     let _ = conn.tx.send(INVALID_SESSION.clone()).await;
// }

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

#[derive(Default)]
pub struct RoleCache {
    roles: HashSet<(Snowflake, Snowflake)>,
}

impl RoleCache {
    fn has(&self, party_id: Snowflake, role_id: Snowflake) -> bool {
        self.roles.contains(&(party_id, role_id))
    }

    fn remove_party(&mut self, party_id: Snowflake) {
        self.roles.retain(|&(pid, _)| pid != party_id);
    }

    fn remove_role(&mut self, party_id: Snowflake, role_id: Snowflake) {
        self.roles.remove(&(party_id, role_id));
    }

    fn add<'a>(&mut self, party_id: Snowflake, role_ids: impl IntoIterator<Item = &'a Snowflake>) {
        self.roles.extend(role_ids.into_iter().map(|&rid| (party_id, rid)));
    }
}
