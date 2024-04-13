use std::{borrow::Cow, net::IpAddr, time::Duration};

use futures::{
    future,
    stream::{self, AbortHandle, BoxStream, SelectAll},
    FutureExt, SinkExt, StreamExt,
};

use tokio_stream::wrappers::ReceiverStream;

use hashbrown::{HashMap, HashSet};

use ftl::ws::{Message as WsMessage, SinkError, WebSocket};
use schema::Snowflake;

use sdk::{
    api::gateway::GatewayQueryParams,
    driver::Encoding,
    models::{
        gateway::message::{ClientMsg, ServerMsg},
        Permissions,
    },
};

use crate::prelude::*;

use super::event::{self as events, Event, EventInner};

pub mod item;
pub mod listener_table;
pub mod role_cache;
pub mod util;

use item::{EventError, Item, MessageIncomingError, MessageOutgoingError};

pub fn client_connected(ws: WebSocket, query: GatewayQueryParams, addr: IpAddr, state: ServerState) {
    tokio::spawn(client_connection(ws, query, addr, state));
}

pub async fn client_connection(ws: WebSocket, query: GatewayQueryParams, _addr: IpAddr, state: ServerState) {
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
                EventInner::External(ref event) => WsMessage::binary(event.get_encoded(7).unwrap().get(query).clone()),
                _ => unreachable!(),
            },
        })
    });

    // for each party that is being listened on, keep the associated cancel handle, to kill the stream if we unsub from them
    let mut listener_table = listener_table::ListenerTable::new();

    // Contains a list of user ids that have blocked the current user of this connection
    let mut blocked_by: HashSet<Snowflake> = HashSet::default();
    let mut roles = role_cache::RoleCache::default();

    // aggregates all event streams into one
    let mut events: SelectAll<BoxStream<Item>> = SelectAll::<BoxStream<Item>>::new();

    // Push Hello event to begin stream and forward ws_rx/conn_rx into events
    events.push(stream::once(future::ready(Item::Event(Ok(events::HELLO_EVENT.clone())))).boxed());
    events.push(ws_rx.boxed());
    events.push(conn_rx.map(|msg| Item::Event(Ok(msg))).boxed());

    let mut user_id: Option<Snowflake> = None;
    let mut intent = sdk::models::Intent::empty();
}
