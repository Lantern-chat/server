use std::{net::SocketAddr, sync::Arc};

use futures::{future, Future, FutureExt, StreamExt};

use tokio::sync::mpsc;
use tokio_postgres::Socket;

use warp::ws::{Message as WsMessage, WebSocket};
use warp::{Filter, Rejection, Reply};

use crate::server::ServerState;

pub mod msg;

pub struct ClientConnection {
    pub addr: Option<SocketAddr>,
    pub tx: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>,
}

pub fn gateway(
    state: Arc<ServerState>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("gateway")
        .and(warp::ws())
        .and(warp::addr::remote())
        .and(warp::any().map(move || state.clone()))
        .map(|ws: warp::ws::Ws, addr, state| {
            ws.on_upgrade(move |socket| client_connected(socket, addr, state))
        })
}

pub async fn client_connected(ws: WebSocket, addr: Option<SocketAddr>, state: Arc<ServerState>) {
    let (ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(rx.forward(ws_tx).map(move |result| match result {
        Err(e) => {} //log::error!("websocket send error: {} to client {}", e, client_id),
        _ => {}
    }));
}
