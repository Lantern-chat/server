use std::{borrow::Cow, net::IpAddr, time::Duration};

use futures::{
    future,
    stream::{self, AbortHandle, BoxStream, SelectAll},
    FutureExt, Sink, SinkExt, StreamExt,
};

use tokio_stream::wrappers::ReceiverStream;

use hashbrown::{hash_map::Entry, HashMap, HashSet};

use ftl::ws::{Message as WsMessage, SinkError, WebSocket};

use sdk::{
    api::gateway::GatewayQueryParams,
    driver::Encoding,
    models::{
        gateway::message::{ClientMsg, ServerMsg},
        Permissions,
    },
};

use crate::{gateway::event::InternalEvent, prelude::*};

use super::{
    conn::GatewayConnection,
    event::{self as events, Event, EventInner, ExternalEvent},
};

pub mod item;
pub mod listener_table;
pub mod role_cache;
pub mod util;

use item::{Item, MessageIncomingError, MessageOutgoingError};

pub fn client_connected(ws: WebSocket, query: GatewayQueryParams, addr: IpAddr, state: GatewayServerState) {
    tokio::spawn(client_connection(ws, query, addr, state));
}

pub struct ConnectionState {
    pub state: GatewayServerState,
    pub conn: GatewayConnection,

    /// for each party that is being listened on, keep the associated cancel handle, to kill the stream if we unsub from them
    pub listener_table: listener_table::ListenerTable,

    /// Contains a list of user ids that have blocked the current user of this connection
    pub blocked_by: HashSet<UserId, sdk::FxRandomState2>,

    pub roles: role_cache::RoleCache,
    pub user_id: Option<UserId>,
    pub intent: sdk::models::Intent,
    pub perm_cache: HashMap<RoomId, Permissions, sdk::FxRandomState2>,
}

impl ConnectionState {
    pub async fn get_perm(&mut self, user_id: UserId, room_id: RoomId) -> Option<Permissions> {
        Some(match self.perm_cache.entry(room_id) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let perms = self.state.gateway.structure.compute_permissions_slow(room_id, user_id).await?;

                *entry.insert(perms)
            }
        })
    }
}

pub enum Loop<T> {
    Continue,
    Yield(T),
    Break,
}

pub async fn client_connection(ws: WebSocket, query: GatewayQueryParams, _addr: IpAddr, state: GatewayServerState) {
    let (ws_tx, ws_rx) = ws.split();

    let (conn, conn_rx) = state.new_gateway_connection().await;
    let conn_rx = ReceiverStream::new(conn_rx);

    // map each incoming websocket message such that it will decompress/decode the message
    // AND update the last_msg value concurrently.
    let conn2 = conn.clone();
    let ws_rx = ws_rx.map(|msg| (msg, &conn2)).then(move |(msg, conn)| async move {
        match msg {
            Err(e) => Item::Msg(Err(MessageIncomingError::from(e))),
            Ok(msg) if msg.is_close() => Item::Msg(Err(MessageIncomingError::SocketClosed)),
            Ok(msg) if msg.is_ping() => {
                conn.heartbeat();

                Item::Ping
            }
            Ok(msg) => {
                // Block to decompress and parse
                let block = tokio::task::spawn_blocking(move || -> Result<_, MessageIncomingError> {
                    let msg = util::decompress_if(query.compress, msg.as_bytes())?;

                    Ok(match query.encoding {
                        Encoding::JSON => serde_json::from_slice(&msg)?,
                        Encoding::CBOR => ciborium::de::from_reader(&msg[..])?,
                    })
                });

                conn.heartbeat();

                match block.await {
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
                // TODO: Don't unwrap, re-evaluate if the encoded event should even be received here?
                EventInner::External(ref event) => WsMessage::binary(event.get_encoded(7).unwrap().get(query)),
                _ => unreachable!(),
            },
        })
    });

    // aggregates all event streams into one
    let mut events: SelectAll<BoxStream<Item>> = SelectAll::<BoxStream<Item>>::new();

    // Push Hello event to begin stream and forward ws_rx/conn_rx into events
    events.push(stream::once(future::ready(Item::Event(Ok(events::HELLO_EVENT.clone())))).boxed());
    events.push(ws_rx.boxed());
    events.push(conn_rx.map(|msg| Item::Event(Ok(msg))).boxed());

    let mut cstate = ConnectionState {
        conn,
        state: state.clone(),
        listener_table: listener_table::ListenerTable::default(),
        blocked_by: HashSet::default(),
        roles: role_cache::RoleCache::default(),
        user_id: None,
        intent: sdk::models::Intent::empty(),
        perm_cache: HashMap::default(),
    };

    'event_loop: while let Some(event) = events.next().await {
        let resp = match cstate.handle_item(event, &mut events).await {
            Loop::Yield(res) => res,
            Loop::Continue => continue 'event_loop,
            Loop::Break => {
                events.clear(); // drop streams early before flushing
                break 'event_loop;
            }
        };

        // group together
        let flush_and_send = async {
            ws_tx.flush().await?;
            ws_tx.send(resp).await
        };

        // TODO: heartbeat timeout should be configurable
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

    if let Some(user_id) = cstate.user_id {
        // if there was a user_id, that means the connection had been readied and a presence possibly set,
        // so kick off a task that will clear the presence after 5 seconds.
        //
        // 5 seconds would give enough time for a page reload, so if the user starts a new connection before then
        // we can avoid flickering presences
        let conn_id = cstate.conn.id;
        let state = state.clone();
        tokio::spawn(async move {
            tokio::time::sleep(state.config().shared.presence_timeout).await;

            //if let Err(e) = clear_presence(state, user_id, conn_id).await {
            //    log::error!("Error clearing connection presence: {e}");
            //}
        });
    }

    // remove connection from gateway tables
    state.gateway.remove_connection(cstate.conn.id, cstate.user_id).await;
}

impl ConnectionState {
    pub async fn handle_event(&mut self, mut event: Event, events: &mut SelectAll<BoxStream<'_, Item>>) -> Loop<Result<Event, MessageOutgoingError>> {
        let e = match *event {
            EventInner::External(ref e) => e,
            EventInner::Internal(ref event) => {
                match event {
                    InternalEvent::BulkUserBlockedRefresh { blocked } => {
                        self.blocked_by.clear();
                        self.blocked_by.extend(blocked);
                    }
                    InternalEvent::UserBlockedAdd { user_id } => {
                        self.blocked_by.insert(*user_id);
                    }
                    InternalEvent::UserBlockedRemove { user_id } => {
                        self.blocked_by.remove(user_id);
                    }
                }

                return Loop::Continue;
            }
        };

        // if this message corresponds to an intent, filter it
        if let Some(matching_intent) = e.msg.matching_intent() {
            if !self.intent.contains(matching_intent) {
                return Loop::Continue; // skip doing anything with this event
            }
        }

        if let Some(user_id) = e.msg.user_id() {
            if self.blocked_by.contains(&user_id) {
                // TODO: Replace message create with "message unavailable" create
                return Loop::Continue; // skip sending this to a user that's been blocked
            }
        }

        // Honestly these are just grouped together to reduce indentation
        match (self.user_id, &e.msg) {
            (_, ServerMsg::Hello(_)) => {}
            (_, ServerMsg::InvalidSession(_)) => {
                events.clear();
                return Loop::Continue;
            }
            (_, ServerMsg::Ready(ref ready)) => {
                self.user_id = Some(ready.user.id);

                for party in &ready.parties {
                    self.roles.add(party.id, &party.me.roles);
                }

                #[rustfmt::skip]
                let subs = self.state.gateway.sub_and_activate_connection(
                    ready.user.id,
                    self.conn.clone(),
                    ready.parties.iter().map(|p| p.id),
                    ready.rooms.iter().map(|r| r.id),
                );

                self.listener_table.register_subs(events, subs.boxed().await);
            }
            (None, _) => {
                log::warn!("Attempted to receive events before user_id was set");
                return Loop::Break;
            }

            // for other events, session must be authenticated and have permission to view such events
            (Some(user_id), _) => {
                self.maybe_clear_cache(e, user_id);

                if let Some(room_id) = e.room_id {
                    // skip event if user can't view room
                    if !matches!(self.get_perm(user_id, room_id).await, Some(perms) if perms.contains(Permissions::VIEW_ROOM)) {
                        return Loop::Continue;
                    }
                }

                match e.msg {
                    ServerMsg::PartyCreate(ref payload) => {
                        let subs = self.state.gateway.sub_and_activate_connection(user_id, self.conn.clone(), [payload.id], []).boxed().await;

                        self.listener_table.register_subs(events, subs);
                    }
                    ServerMsg::RoomCreate(ref payload) => {
                        todo!();
                        //let subs = self.state.gateway.sub_and_activate_connection(user_id, self.conn.clone(), [], [payload.id]).boxed().await;
                        //self.listener_table.register_subs(events, subs);
                    }
                    ServerMsg::PartyDelete(ref payload) => {
                        // by cancelling a stream, it will be removed from the SelectStream automatically
                        self.listener_table.get(&payload.id).map(|event_stream| event_stream.abort());
                    }
                    ServerMsg::RoomDelete(ref payload) => {
                        self.listener_table.get(&payload.id).map(|event_stream| event_stream.abort());
                    }
                    _ => {}
                }
            }
        }

        Loop::Yield(Ok(event)) // forward event directly to tx
    }

    /// Check if the event should clear the permission cache due to changing the underlying structure
    pub fn maybe_clear_cache(&mut self, e: &ExternalEvent, user_id: UserId) {
        use sdk::models::gateway::message::server_msg_payloads::{MemberRemovePayload, MemberUpdatePayload};

        // if the event indicates things that can invalidate the permission cache, they must be handled
        #[rustfmt::skip]
        let clear_cache = match e.msg {
            // role updates when the current user has this role
            ServerMsg::RoleUpdate(ref r) if self.roles.has(r.party_id, r.id) => true,
            ServerMsg::RoleDelete(ref r) if self.roles.has(r.party_id, r.id) => {
                self.roles.remove_role(r.party_id, r.id);
                true
            }
            // member events for the current user
            ServerMsg::MemberUpdate(MemberUpdatePayload { ref inner }) |
            ServerMsg::MemberRemove(MemberRemovePayload { ref inner }) if inner.member.user.id == user_id => {
                // remove old roles and add new
                self.roles.remove_party(inner.party_id);

                self.roles.add(inner.party_id, &inner.member.roles);

                true
            }
            ServerMsg::PartyDelete(ref p) => {
                self.roles.remove_party(p.id);
                true
            }
            ServerMsg::RoomUpdate(ref _r) => {
                // TODO: self.perm_cache.remove(_r.id);
                // false

                true
            }
            _ => false,
        };

        if clear_cache {
            self.perm_cache.clear();
        }
    }

    pub async fn handle_msg(&mut self, msg: ClientMsg, events: &mut SelectAll<BoxStream<'_, Item>>) -> Loop<Result<Event, MessageOutgoingError>> {
        match msg {
            // Respond to heartbeats immediately.
            ClientMsg::Heartbeat(_) => Loop::Yield(Ok(events::HEARTBEAT_ACK.clone())),

            ClientMsg::Identify(payload) => {
                // this will send a ready event on success
                //tokio::spawn(identify::identify(state.clone(), conn.clone(), payload.inner.auth, payload.intent));
                self.intent = payload.intent;
                Loop::Continue
            }
            ClientMsg::Resume(_) => {
                log::error!("Attempted to resume connection");
                Loop::Break
            }
            ClientMsg::SetPresence(payload) => {
                match self.user_id {
                    None => {
                        log::warn!("Attempted to set presence before identification");
                        return Loop::Break;
                    }
                    Some(_user_id) => {
                        //tokio::spawn(set_presence(state.clone(), user_id, conn.id, payload.inner.presence));
                    }
                }

                Loop::Continue // no reply, so continue event loop
            }
            ClientMsg::Subscribe(_) | ClientMsg::Unsubscribe(_) => {
                log::error!("Unimplemented sub/unsub");
                Loop::Continue // no reply
            }
        }
    }

    pub async fn handle_item(&mut self, event: Item, events: &mut SelectAll<BoxStream<'_, Item>>) -> Loop<Result<Event, MessageOutgoingError>> {
        match event {
            Item::Event(Ok(event)) => self.handle_event(event, events).await,
            Item::Msg(Ok(msg)) => self.handle_msg(msg, events).await,

            Item::MissedHeartbeat => Loop::Yield(Err(MessageOutgoingError::SocketClosed)),

            // Pong resposes should be handled by the underlying socket,
            // but we still need to ignore the message
            Item::Ping => Loop::Continue,

            Item::Event(Err(e)) => {
                log::warn!("Event error: {e}");

                Loop::Yield(Err(MessageOutgoingError::SocketClosed)) // kick for lag?
            }

            Item::Msg(Err(e)) => match e {
                _ if e.is_close() => {
                    log::warn!("Connection disconnected");
                    Loop::Break
                }
                // TODO: Send code with it
                _ => {
                    log::error!("Misc err: {e}");
                    Loop::Yield(Err(MessageOutgoingError::SocketClosed))
                }
            },
        }
    }
}
