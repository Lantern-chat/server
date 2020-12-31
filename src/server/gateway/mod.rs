use std::{borrow::Cow, error::Error, net::SocketAddr, sync::Arc};

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
    pub encoding: GatewayMsgEncoding,

    /// Whether to compress individual messages
    #[serde(default = "default_compress")]
    pub compress: bool,
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
    let (tx, rx) = warp::test::unbounded_channel_stream();

    tokio::spawn(rx.forward(ws_tx).map(move |result: Result<_, _>| {
        result.map_err(|e| log::error!("websocket send error: {} to client {:?}", e, addr))
    }));

    let send = |msg: msg::Message| -> Result<(), Box<dyn Error>> {
        let mut msg: Vec<u8> = match query.encoding {
            GatewayMsgEncoding::Json => serde_json::to_vec(&msg)?,
            GatewayMsgEncoding::MsgPack => rmp_serde::to_vec_named(&msg)?,
            _ => unimplemented!(),
        };

        if query.compress {
            use flate2::{write::ZlibEncoder, Compression};
            use std::io::Write;

            let mut encoder = ZlibEncoder::new(Vec::with_capacity(128), Compression::new(6));
            encoder.write(&msg)?;
            msg = encoder.finish()?;
        }

        tx.send(Ok(WsMessage::binary(msg))).map_err(Into::into)
    };

    let mut initiated = true;

    if let Err(e) = send(Message::new_hello(45000)) {
        log::error!("Unable to send Hello message: {:?}", e);
        initiated = false;
    }

    // TODO: Register connected client for subscriptions

    // default to forceful disconnection which is overridden for safe disconnects
    let mut force_disconnect = true;

    while initiated {
        match ws_rx.next().await {
            // if None was received, we can assume the websocket safely closed
            None => force_disconnect = false,

            Some(Ok(msg)) => {
                let mut msg = Cow::Borrowed(msg.as_bytes());

                if query.compress {
                    use flate2::bufread::ZlibDecoder;
                    use std::io::Read;

                    let mut reader = ZlibDecoder::new(&*msg);
                    let mut decoded = Vec::with_capacity(128);

                    if let Err(e) = reader.read_to_end(&mut decoded) {
                        log::error!("Invalid decompression: {:?}", e);

                        break;
                    }

                    msg = Cow::Owned(decoded);
                }

                let msg: Result<Message, Box<dyn Error>> = match query.encoding {
                    GatewayMsgEncoding::Json => serde_json::from_slice(&msg).map_err(Into::into),
                    GatewayMsgEncoding::MsgPack => rmp_serde::from_slice(&msg).map_err(Into::into),
                    _ => unimplemented!(),
                };

                let msg = match msg {
                    Ok(msg) => msg,
                    Err(e) => {
                        log::error!("Error parsing incoming message: {:?}", e);
                        break;
                    }
                };

                // TODO: Place elsewhere
                let res = send(match msg {
                    Message::Heartbeat { .. } => Message::new_heartbeatack(),
                    _ => unimplemented!(),
                });

                if let Err(e) = res {
                    log::error!("Error sending message response: {:?}", e);
                    break;
                }

                continue;
            }
            Some(Err(e)) => log::error!("Receiving websocket message: {}", e),
        }

        break;
    }

    // TODO: Disconnect client
}
