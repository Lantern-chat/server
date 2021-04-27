use std::{net::SocketAddr, sync::Arc};

use futures::{future, Future, FutureExt, StreamExt};

use tokio::sync::mpsc;
use tokio_postgres::Socket;

use warp::ws::{Message as WsMessage, WebSocket};
use warp::{Filter, Rejection, Reply};

use crate::ServerState;

use crate::gateway::{client_connected, GatewayQueryParams};

use super::filters::real_ip;

pub fn gateway(
    state: ServerState,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path("gateway")
        .and(warp::ws())
        .and(warp::query::<GatewayQueryParams>())
        .and(real_ip())
        .map(move |ws: warp::ws::Ws, query: GatewayQueryParams, addr| {
            let state = state.clone();

            ws.on_upgrade(move |socket| client_connected(socket, query, addr, state))
        })
}
