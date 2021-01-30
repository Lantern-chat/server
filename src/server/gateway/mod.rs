use std::{borrow::Cow, error::Error, net::SocketAddr, sync::Arc};

use futures::{future, Future, FutureExt, StreamExt};

use tokio::sync::mpsc;
use tokio_postgres::Socket;
use tokio_stream::wrappers::UnboundedReceiverStream;

use warp::ws::{Message as WsMessage, WebSocket};
use warp::{Filter, Rejection, Reply};

use crate::{
    db::Snowflake,
    server::{rate::RateLimiter, ServerState},
};

pub mod msg;
use msg::{ClientMsg, ServerMsg};

pub mod conn;

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

const MSGS_PER_SEC: f32 = 50.0;

pub enum ClienState {
    Hello,
    Identified(Snowflake),
}

pub async fn client_connected(
    ws: WebSocket,
    query: GatewayQueryParams,
    addr: Option<SocketAddr>,
    state: Arc<ServerState>,
) {
    let (ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(UnboundedReceiverStream::new(rx).forward(ws_tx).map(
        move |result: Result<_, _>| {
            result.map_err(|e| log::error!("websocket send error: {} to client {:?}", e, addr))
        },
    ));

    // helper to encode and compress messages before sending
    let send = |msg: ServerMsg| -> anyhow::Result<()> {
        Ok(tx.send(Ok(WsMessage::binary(compress_if(
            query.compress,
            match query.encoding {
                GatewayMsgEncoding::Json => serde_json::to_vec(&msg)?,
                GatewayMsgEncoding::MsgPack => rmp_serde::to_vec_named(&msg)?,
            },
        )?)))?)
    };

    let mut initiated = true;

    if let Err(e) = send(ServerMsg::new_hello(45000)) {
        log::error!("Unable to send Hello message: {:?}", e);
        initiated = false;
    }

    // TODO: Register connected client for subscriptions

    // default to forceful disconnection which is overridden for safe disconnects
    let mut force_disconnect = true;

    // rate-limit the websocket message stream
    let mut rate_limiter = RateLimiter::default();

    // if initiated then loop {}
    while initiated {
        match ws_rx.next().await {
            // if None was received, we can assume the websocket safely closed
            None => force_disconnect = false,
            Some(Ok(msg)) if msg.is_close() => break,
            Some(Ok(msg)) => {
                // First, check rate limiting and kick if needed
                if !rate_limiter.update(MSGS_PER_SEC) {
                    break; // kick
                }

                // decompress message
                let msg = match decompress_if(query.compress, msg.as_bytes()) {
                    Ok(msg) => msg,
                    Err(e) => {
                        log::error!("Invalid decompression: {:?}", e);
                        break;
                    }
                };

                // parse message
                let msg: anyhow::Result<ClientMsg> = match query.encoding {
                    GatewayMsgEncoding::Json => serde_json::from_slice(&msg).map_err(Into::into),
                    GatewayMsgEncoding::MsgPack => rmp_serde::from_slice(&msg).map_err(Into::into),
                };

                // kick on error
                let msg = match msg {
                    Ok(msg) => msg,
                    Err(e) => {
                        log::error!("Error parsing incoming message: {:?}", e);
                        break;
                    }
                };

                // TODO: Place elsewhere
                let res = send(unimplemented!());

                if let Err(e) = res {
                    log::error!("Error sending message response: {:?}", e);
                    break;
                }

                continue; // don't kick, continue receiving messages
            }
            Some(Err(e)) => log::error!("Receiving websocket message: {}", e),
        }

        break;
    }

    // TODO: Disconnect client
}

fn compress_if(cond: bool, mut msg: Vec<u8>) -> std::io::Result<Vec<u8>> {
    if !cond {
        return Ok(msg);
    }

    use flate2::{write::ZlibEncoder, Compression};
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::with_capacity(128), Compression::new(6));
    encoder.write(&msg)?;
    encoder.finish()
}

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

pub async fn process_message(
    state: Arc<ServerState>,
    msg: ClientMsg,
) -> Result<ServerMsg, anyhow::Error> {
    unimplemented!()
}
