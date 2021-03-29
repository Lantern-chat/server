use std::{borrow::Cow, error::Error, net::IpAddr, pin::Pin, sync::Arc, time::Instant};

use futures::{future, Future, FutureExt, SinkExt, StreamExt, TryStreamExt};

use tokio::sync::mpsc;
use tokio_postgres::Socket;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{
    db::Snowflake,
    server::ftl::ws::{Message as WsMessage, WebSocket},
};

use crate::server::{routes::api::auth::Authorization, ServerState};

use super::msg::{ClientMsg, ServerMsg};

pub mod params;
pub use params::{GatewayMsgEncoding, GatewayQueryParams};

pub fn client_connected(
    ws: WebSocket,
    query: GatewayQueryParams,
    addr: IpAddr,
    state: ServerState,
) {
    tokio::spawn(async move {
        let (ws_tx, mut ws_rx) = ws.split();
    });
}
