use std::{net::SocketAddr, sync::Arc};

use futures::{future, Future, FutureExt, StreamExt};

use tokio::sync::mpsc;
use tokio_postgres::Socket;

use warp::ws::{Message, WebSocket};

use crate::state::ServerState;

pub struct ClientConnection {
    pub addr: Option<SocketAddr>,
    pub tx: mpsc::UnboundedSender<Result<Message, warp::Error>>,
}

pub async fn client_connected(ws: WebSocket, addr: Option<SocketAddr>, state: Arc<ServerState>) {
    let (ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(rx.forward(ws_tx).map(move |result| match result {
        Err(e) => {} //log::error!("websocket send error: {} to client {}", e, client_id),
        _ => {}
    }));
}
