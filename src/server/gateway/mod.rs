use std::{borrow::Cow, error::Error, net::IpAddr, pin::Pin, sync::Arc, time::Instant};

use futures::{future, Future, FutureExt, SinkExt, StreamExt, TryStreamExt};

use tokio::sync::mpsc;
use tokio_postgres::Socket;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{
    db::Snowflake,
    server::ftl::ws::{Message as WsMessage, WebSocket},
};

pub mod msg;
use msg::{ClientMsg, ServerMsg};

use super::{routes::api::auth::Authorization, ServerState};

/// Websocket message encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GatewayMsgEncoding {
    /// Textual JSON, simple.
    Json,

    /// Binary MessagePack (smaller, slower to encode/decode in browser)
    ///
    /// This is recommended when you have access to natively compiled MsgPack libraries
    MsgPack,
}

impl Default for GatewayMsgEncoding {
    fn default() -> Self {
        GatewayMsgEncoding::Json
    }
}

const fn default_compress() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GatewayQueryParams {
    /// Encoding method for each individual websocket message
    #[serde(default)]
    pub encoding: GatewayMsgEncoding,

    /// Whether to compress individual messages
    #[serde(default = "default_compress")]
    pub compress: bool,
}

impl Default for GatewayQueryParams {
    fn default() -> Self {
        GatewayQueryParams {
            encoding: GatewayMsgEncoding::default(),
            compress: default_compress(),
        }
    }
}

pub mod conn;


//pub enum ClientState {
//    Hello,
//    Identified(Box<ClientConnection>),
//}

pub fn client_connected(
    ws: WebSocket,
    query: GatewayQueryParams,
    addr: IpAddr,
    state: ServerState,
) {
    /*
    tokio::spawn(async move {
        log::info!("Client Connected!");

        let (ws_tx, ws_rx) = ws.split();

        let mut ws_rx = ws_rx.map(|msg| {
            // TODO
            ()
        });

        futures::pin_mut!(ws_rx);

        let mut state = ClientState::Hello;

        loop {
            match state {
                ClientState::Hello => {}
                ClientState::Identified(ref mut conn) => {
                    let ev_rx = unsafe { Pin::new_unchecked(&mut conn.ev_rx) };

                    let msg: _ = futures::stream::select(ws_rx, ev_rx).await;
                }
            }

            break;
        }
    });

    ws_rx
        .then(|msg| async move {
            // TODO
            unimplemented!()
        })
        .forward(ws_tx);
        */

    //ws_rx.flat_map(|msg| {});
}
