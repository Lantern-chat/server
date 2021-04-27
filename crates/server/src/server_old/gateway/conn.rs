use std::{borrow::Cow, error::Error, net::SocketAddr, sync::Arc};

use futures::{future, Future, FutureExt, StreamExt};

use tokio::sync::mpsc;
use tokio_postgres::Socket;

use warp::ws::{Message as WsMessage, WebSocket};
use warp::{Filter, Rejection, Reply};

use crate::{events::Event, rate::RateLimiter, ServerState};

use super::msg::{ClientMsg, ServerMsg};

pub struct ClientConnection {
    pub id: usize,
    pub addr: Option<SocketAddr>,
    pub tx: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>,
}

impl ClientConnection {
    pub async fn on_msg(&self, msg: ClientMsg) -> anyhow::Result<ServerMsg> {
        Ok(match msg {
            ClientMsg::Heartbeat { .. } => ServerMsg::new_heartbeatack(),
            _ => unimplemented!(),
        })
    }

    pub async fn process_event(&self, event: Arc<Event>) {}
}
