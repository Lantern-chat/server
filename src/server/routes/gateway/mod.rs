use std::{net::SocketAddr, sync::Arc};

use futures::{future, Future, FutureExt, StreamExt};

use tokio::sync::mpsc;
use tokio_postgres::Socket;

use warp::ws::{Message as WsMessage, WebSocket};
use warp::{Filter, Rejection, Reply};

use crate::server::ServerState;

pub mod msg;
use msg::Message;

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
    encoding: GatewayMsgEncoding,

    /// Whether to compress individual messages
    #[serde(default = "default_compress")]
    compress: bool,
}

pub fn gateway(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("gateway")
        .and(warp::ws())
        .and(warp::query::<GatewayQueryParams>())
        .and(warp::addr::remote())
        .map(move |ws: warp::ws::Ws, query: GatewayQueryParams, addr| {
            let state = state.clone();

            ws.on_upgrade(move |socket| client_connected(socket, query, addr, state))
        })
}

pub struct ClientConnection {
    pub query: GatewayQueryParams,
    pub addr: Option<SocketAddr>,
    pub tx: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>,
}

pub async fn client_connected(
    ws: WebSocket,
    query: GatewayQueryParams,
    addr: Option<SocketAddr>,
    state: Arc<ServerState>,
) {
    let (ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(rx.forward(ws_tx).map(move |result: Result<_, _>| {
        result.map_err(|e| log::error!("websocket send error: {} to client {:?}", e, addr))
    }));

    let send = |msg: msg::Message| -> Result<(), _> {
        // TODO: Monomorphize?
        match (query.encoding, query.compress) {
            (GatewayMsgEncoding::Json, false) => {
                tx.send(Ok(WsMessage::text(serde_json::to_string(&msg).unwrap())))
            }
            _ => unimplemented!(),
        }
    };

    let mut initiated = true;

    if let Err(e) = send(Message::new_hello(45000)) {
        log::error!("Unable to send Hello message: {:?}", e);
        initiated = false;
    }

    // default to forceful disconnection which is overridden for safe disconnects
    let mut force_disconnect = true;

    while initiated {
        match ws_rx.next().await {
            Some(Ok(msg)) => {
                let msg = match (query.encoding, query.compress) {
                    (GatewayMsgEncoding::Json, false) if msg.is_text() => {
                        serde_json::from_str(msg.to_str().unwrap())
                    }
                    (GatewayMsgEncoding::Json, false) if msg.is_binary() => {
                        serde_json::from_slice(msg.as_bytes())
                    }
                    _ => unimplemented!(),
                };

                let msg = match msg {
                    Ok(msg) => msg,
                    Err(e) => {
                        log::error!("Error parsing incoming message: {:?}", e);
                        break;
                    }
                };

                let res = match msg {
                    Message::Heartbeat { .. } => send(Message::new_heartbeatack()),
                    _ => unimplemented!(),
                };

                if let Err(e) = res {
                    log::error!("Error sending message response: {:?}", e);
                    break;
                }

                continue;
            }
            Some(Err(e)) => log::error!("Receiving websocket message: {}", e),

            // if None was received, we can assume the websocket safely closed
            None => force_disconnect = false,
        }

        break;
    }

    // TODO: Disconnect client
}
