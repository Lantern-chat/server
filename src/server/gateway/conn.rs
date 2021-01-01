use std::{borrow::Cow, error::Error, net::SocketAddr, sync::Arc};

use futures::{future, Future, FutureExt, StreamExt};

use tokio::sync::mpsc;
use tokio_postgres::Socket;

use warp::ws::{Message as WsMessage, WebSocket};
use warp::{Filter, Rejection, Reply};

use crate::server::{rate::RateLimiter, ServerState};

use super::msg::Message;

pub struct ClientConnection {
    pub id: usize,
    pub addr: Option<SocketAddr>,
    pub tx: mpsc::UnboundedSender<Result<WsMessage, warp::Error>>,
}

impl ClientConnection {
    pub async fn on_msg(&self, msg: Message) -> Result<Message, Box<dyn Error>> {
        Ok(match msg {
            Message::Heartbeat { .. } => Message::new_heartbeatack(),
            _ => unimplemented!(),
        })
    }
}
