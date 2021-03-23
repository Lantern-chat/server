use std::{
    borrow::Cow, collections::BTreeMap, error::Error, net::IpAddr, pin::Pin, sync::Arc,
    time::Instant,
};

use futures::{future, Future, FutureExt, SinkExt, StreamExt, TryStreamExt};

use tokio::sync::{mpsc, RwLock};
use tokio_postgres::Socket;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::{db::Snowflake, util::cmap::CHashMap};

pub struct ClientConnection {
    pub user_id: Snowflake,
    pub params: super::GatewayQueryParams,
    pub ev_rx: mpsc::Receiver<()>,
    // TODO: Expire the session when applicable
    //pub expires:
    // NOTE: Not needed since it's already authorized
    //pub auth: Authorization,
}

pub type PartyId = Snowflake;
pub type UserId = Snowflake;
pub type EventId = Snowflake;
pub type ClientId = Snowflake;

pub struct EventInner {}

#[derive(Clone)]
pub struct Event(Arc<EventInner>);

pub struct Subscriber {}

pub struct SubscriberMap {
    pub subs: CHashMap<ClientId, Subscriber>,
}

pub struct ConnectionTable {
    pub events: CHashMap<PartyId, SubscriberMap>,
    //pub conns: CHashMap,
}
