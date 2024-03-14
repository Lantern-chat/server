use std::{
    borrow::Cow,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use futures_util::{FutureExt, Stream, StreamExt};
use indexmap::IndexMap;
use parking_lot::RwLock;

use quinn::{Connection, Endpoint};
use sdk::Snowflake;

#[derive(Debug, thiserror::Error)]
pub enum RpcClientError {
    #[error("RPC Room association not found: {0}")]
    MissingRoom(Snowflake),

    #[error("RPC Party association not found: {0}")]
    MissingParty(Snowflake),

    #[error("RPC Faction Server not found: {0}")]
    MissingFaction(Snowflake),

    #[error("ConnectError: {0}")]
    Connect(#[from] quinn::ConnectError),

    #[error("ConnectionError: {0}")]
    Connection(#[from] quinn::ConnectionError),
}

#[derive(Debug, Clone)]
pub struct RpcClientConnection {
    conn: Connection,
}

pub struct RpcClient {
    addr: SocketAddr,
    name: Cow<'static, str>,
    max_conns: usize,
    counter: AtomicU64,
    endpoint: Endpoint,
    conns: RwLock<IndexMap<u64, RpcClientConnection>>,
}

pub struct RpcManager {
    nexus: Arc<RpcClient>,
    factions: scc::HashMap<Snowflake, Arc<RpcClient>>,
    rooms: scc::HashIndex<Snowflake, Snowflake>,
}

use crate::msg::Resolve;

impl RpcManager {
    pub async fn add_rooms(&self, rooms: impl Stream<Item = (Snowflake, Snowflake)>) {
        #[rustfmt::skip]
        let _ = rooms.for_each_concurrent(16, |(room_id, party_id)| {
            self.rooms.insert_async(room_id, party_id).map(|_| ())
        }).await;
    }

    pub async fn add_faction(&self, faction_id: Snowflake, client: RpcClient) -> Arc<RpcClient> {
        use scc::hash_map::Entry;

        let mut client = Arc::new(client);

        match self.factions.entry_async(faction_id).await {
            Entry::Vacant(v) => _ = v.insert_entry(client.clone()),
            Entry::Occupied(o) => client = o.get().clone(),
        }

        client
    }

    pub async fn add_parties(
        &self,
        faction_id: Snowflake,
        parties: impl Stream<Item = Snowflake>,
    ) -> Result<(), RpcClientError> {
        let Some(faction_client) = self.factions.get_async(&faction_id).await else {
            return Err(RpcClientError::MissingFaction(faction_id));
        };

        let client = faction_client.get().clone();

        #[rustfmt::skip]
        let _ = parties.for_each_concurrent(16, |party_id| {
            self.factions.insert_async(party_id, client.clone()).map(|_| ())
        }).await;

        Ok(())
    }

    pub async fn remove_party(&self, party_id: Snowflake) {
        tokio::join!(
            self.factions.remove_async(&party_id),
            self.rooms.retain_async(|_, &pid| pid != party_id),
        );
    }

    pub async fn get_connection(&self, kind: Resolve) -> Result<RpcClientConnection, RpcClientError> {
        self.get_client(kind).await?.get_connection().await
    }

    pub async fn get_client(&self, mut kind: Resolve) -> Result<Arc<RpcClient>, RpcClientError> {
        if let Resolve::Room(room_id) = kind {
            kind = match self.rooms.peek(&room_id, &scc::ebr::Guard::new()) {
                Some(party_id) => Resolve::Party(*party_id),
                None => return Err(RpcClientError::MissingRoom(room_id)),
            };
        }

        Ok(match kind {
            Resolve::Nexus => self.nexus.clone(),
            Resolve::Party(party_id) => match self.factions.get_async(&party_id).await {
                Some(client) => client.get().clone(),
                None => return Err(RpcClientError::MissingParty(party_id)),
            },
            _ => unreachable!(),
        })
    }
}

impl RpcClientConnection {
    pub fn is_closed(&self) -> bool {
        self.conn.close_reason().is_some()
    }
}

impl RpcClient {
    pub fn new(endpoint: Endpoint, addr: SocketAddr, name: impl Into<Cow<'static, str>>) -> RpcClient {
        RpcClient {
            addr,
            name: name.into(),
            max_conns: 2,
            counter: AtomicU64::new(0),
            endpoint,
            conns: RwLock::new(IndexMap::new()),
        }
    }

    pub fn set_max_conns(&mut self, max_conns: usize) {
        self.max_conns = max_conns;
    }

    /// Get an existing connection or connect if below the max_conns threshold.
    pub async fn get_connection(&self) -> Result<RpcClientConnection, RpcClientError> {
        // TODO: what if weirdness where the connections keep immediately closing?
        loop {
            let next = self.counter.fetch_add(1, Ordering::Relaxed);

            let mut conns = self.conns.upgradable_read();

            if conns.len() < self.max_conns {
                let conn = RpcClientConnection {
                    conn: self.endpoint.connect(self.addr, &self.name)?.await?,
                };

                conns.with_upgraded(|conns| conns.insert(next, conn.clone()));

                return Ok(conn);
            }

            if let Some((&conn_id, c)) = conns.get_index(next as usize % conns.len()) {
                if !c.is_closed() {
                    return Ok(c.clone());
                }

                // swap_remove will make things slightly unfair until the next round, but oh well, it's the fastest.
                conns.with_upgraded(|conns| conns.swap_remove(&conn_id));
            }
        }
    }
}
