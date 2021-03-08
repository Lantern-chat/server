use std::{borrow::Cow, error::Error, net::IpAddr, sync::Arc};

use futures::{future, Future, FutureExt, StreamExt};

use tokio::sync::mpsc;
use tokio_postgres::Socket;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::server::ftl::ws::{Message as WsMessage, WebSocket};

pub mod msg;
use msg::{ClientMsg, ServerMsg};

use super::ServerState;

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

pub async fn client_connected(
    ws: WebSocket,
    query: GatewayQueryParams,
    addr: IpAddr,
    state: ServerState,
) {
    log::info!("Client Connected!");
}
