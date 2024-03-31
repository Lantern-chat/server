#![allow(clippy::toplevel_ref_arg)]

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::{borrow::Cow, net::SocketAddr};

use futures_util::{stream::FuturesUnordered, StreamExt};
use indexmap::IndexMap;
use parking_lot::RwLock;

use quinn::{Connection, Endpoint};
use sdk::{api::error::ApiError, Snowflake};
use tokio::io::AsyncWriteExt;

use framed::tokio::AsyncFramedWriter;

use rkyv::{result::ArchivedResult, ser::serializers::AllocSerializer};
use rkyv::{ser::Serializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resolve {
    Nexus,
    Party(Snowflake),
    Room(Snowflake),
}

impl Resolve {
    pub(crate) const fn party(self, id: Snowflake) -> Resolve {
        match self {
            Resolve::Party(_) => self,
            _ => Resolve::Party(id),
        }
    }

    pub(crate) const fn room(self, id: Snowflake) -> Resolve {
        match self {
            Resolve::Room(_) => self,
            _ => Resolve::Room(id),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RpcClientError {
    #[error("IOError: {0}")]
    IOError(#[from] std::io::Error),

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

    #[error("Rkyv Encoding Error")]
    EncodingError,

    #[error("Does Not Exist")]
    DoesNotExist,
}

#[derive(Debug, Clone)]
pub struct RpcClientConnection {
    conn: Connection,
}

pub struct RpcClientInner {
    addr: SocketAddr,
    faction_id: Snowflake,
    name: Cow<'static, str>,
    /// The nominal number of connections to maintain to the server.
    nominal_conns: usize,
    counter: AtomicU64,
    endpoint: Endpoint,
    conns: RwLock<IndexMap<u64, RpcClientConnection>>,
}

/// A client to a remote RPC server.
///
/// Internally an `Arc` is used to allow for multiple references to the same client,
/// but implementing `Hash` and `Eq` using the pointer value to allow for use in hashmaps.
#[derive(Clone)]
#[repr(transparent)]
pub struct RpcClient(Arc<RpcClientInner>);

const _: () = {
    use std::hash::{Hash, Hasher};
    use std::ops::Deref;

    impl Deref for RpcClient {
        type Target = RpcClientInner;
        #[inline]
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl PartialEq for RpcClient {
        #[inline]
        fn eq(&self, other: &Self) -> bool {
            Arc::ptr_eq(&self.0, &other.0)
        }
    }

    impl Eq for RpcClient {}

    impl Hash for RpcClient {
        #[inline]
        fn hash<H: Hasher>(&self, state: &mut H) {
            Arc::as_ptr(&self.0).hash(state);
        }
    }
};

pub struct RpcManager {
    /// User nexus client.
    nexus: RpcClient,

    /// All configured clients, excluding the nexus.
    clients: scc::HashSet<RpcClient>,
    /// Faction clients, with party_id as the key.
    ///
    /// There will be a unique entry for the faction server using its own faction_id.
    factions: scc::HashMap<Snowflake, RpcClient>,
    /// Room to party association.
    rooms: scc::HashIndex<Snowflake, Snowflake>,
}

use crate::request::{PartyInfo, RpcRequest};

impl RpcManager {
    async fn find_faction(&self, endpoint: Resolve) -> Result<Option<RpcClient>, RpcClientError> {
        let mut clients = Vec::new();
        self.clients.scan_async(|c| clients.push(c.clone())).await;

        let ref msg = match endpoint {
            Resolve::Nexus => unreachable!(),
            Resolve::Party(party_id) => RpcRequest::GetPartyInfoFromPartyId(party_id),
            Resolve::Room(room_id) => RpcRequest::GetPartyInfoFromRoomId(room_id),
        };

        let mut futs = FuturesUnordered::from_iter(clients.into_iter().map(|client| async move {
            let mut recv = crate::stream::RpcRecvReader::new(client.send(msg).await?);

            let res = match recv.recv::<Result<PartyInfo, ApiError>>().await? {
                None => return Ok(None),
                Some(ArchivedResult::Ok(res)) => res,
                Some(ArchivedResult::Err(e)) => {
                    log::error!("Remote RPC Error during party lookup: {:?}", e);
                    return Ok(None);
                }
            };

            self.set_party(client.faction_id, res.party_id, res.room_ids.as_ref()).await.map(Some)
        }));

        while let Some(res) = futs.next().await {
            if matches!(res, Ok(Some(_))) {
                return res;
            }
        }

        Ok(None)
    }

    pub async fn send(&self, cmd: RpcRequest) -> Result<quinn::RecvStream, RpcClientError> {
        let client = match cmd {
            RpcRequest::Procedure { ref proc, .. } => {
                let endpoint = proc.endpoint();

                match self.get_client(endpoint).await {
                    Ok(client) => client,
                    Err(RpcClientError::MissingParty(_) | RpcClientError::MissingRoom(_)) => {
                        match self.find_faction(endpoint).await? {
                            Some(client) => client,
                            None => return Err(RpcClientError::DoesNotExist),
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
            _ => unimplemented!("Non-procedure requests"),
        };

        client.send(&cmd).await
    }
}

impl RpcManager {
    pub fn new(nexus: RpcClient) -> RpcManager {
        RpcManager {
            nexus,
            clients: scc::HashSet::new(),
            factions: scc::HashMap::new(),
            rooms: scc::HashIndex::new(),
        }
    }

    /// Add a faction to the manager, returning the client to use for the faction,
    /// or the existing client if it already exists.
    pub async fn add_faction(&self, mut client: RpcClient) -> RpcClient {
        use scc::hash_map::Entry;

        match self.factions.entry_async(client.faction_id).await {
            Entry::Vacant(v) => {
                _ = v.insert_entry(client.clone());
                _ = self.clients.insert_async(client.clone()).await;
            }
            Entry::Occupied(o) => client = o.get().clone(),
        }

        client
    }

    pub async fn set_party<'a, 'b>(
        &'a self,
        faction_id: Snowflake,
        party_id: Snowflake,
        room_ids: &'b [Snowflake],
    ) -> Result<RpcClient, RpcClientError>
    where
        'a: 'b, // don't let the room_ids reference outlive the party_id
    {
        use scc::hash_map::Entry;

        let client = match self.factions.get_async(&faction_id).await {
            Some(faction_client) => faction_client.get().clone(),
            _ => return Err(RpcClientError::MissingFaction(faction_id)),
        };

        // NOTE: This is kind of weird because of async lifetimes and `Send` weirdness.
        _ = tokio::join!(
            async {
                match self.factions.entry_async(party_id).await {
                    Entry::Vacant(v) => _ = v.insert_entry(client.clone()),
                    Entry::Occupied(mut v) => _ = v.insert(client.clone()),
                }
            },
            async {
                futures_util::stream::iter(room_ids)
                    .for_each_concurrent(16, |&room_id| async move {
                        _ = self.rooms.insert_async(room_id, party_id).await;
                    })
                    .await;
            },
        );

        Ok(client)
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

    pub async fn get_client(&self, mut kind: Resolve) -> Result<RpcClient, RpcClientError> {
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
    pub fn new(
        endpoint: Endpoint,
        faction_id: Snowflake,
        addr: SocketAddr,
        max_conns: usize,
        name: impl Into<Cow<'static, str>>,
    ) -> RpcClient {
        RpcClient(Arc::new(RpcClientInner {
            addr,
            faction_id,
            name: name.into(),
            nominal_conns: max_conns,
            counter: AtomicU64::new(0),
            endpoint,
            conns: RwLock::new(IndexMap::new()),
        }))
    }

    /// Attempts to get an existing connection but only if the number of available connections is at or above the max_conns threshold.
    pub fn try_get_connection(&self) -> Option<RpcClientConnection> {
        let mut conns = self.conns.upgradable_read();

        loop {
            if conns.len() < self.nominal_conns {
                return None;
            }

            let next = self.counter.fetch_add(1, Ordering::Relaxed);

            if let Some((&conn_id, c)) = conns.get_index(next as usize % conns.len()) {
                if !c.is_closed() {
                    return Some(c.clone());
                }

                // swap_remove will make things slightly unfair until the next round, but oh well, it's the fastest.
                conns.with_upgraded(|conns| conns.swap_remove(&conn_id));
            }
        }
    }

    /// Get an existing connection or connect if below the max_conns threshold.
    pub async fn get_connection(&self) -> Result<RpcClientConnection, RpcClientError> {
        // TODO: what if weirdness where the connections keep immediately closing?

        if let Some(conn) = self.try_get_connection() {
            return Ok(conn);
        }

        let conn = RpcClientConnection {
            conn: self.endpoint.connect(self.addr, &self.name)?.await?,
        };

        self.conns.write().insert(self.counter.fetch_add(1, Ordering::Relaxed), conn.clone());

        Ok(conn)
    }

    pub async fn send_raw(&self, value: impl AsRef<[u8]>) -> Result<quinn::RecvStream, RpcClientError> {
        let conn = self.get_connection().await?;
        let (send, recv) = conn.conn.open_bi().await?;

        let mut out = AsyncFramedWriter::new(send);

        let mut msg = out.new_message();
        msg.write_all(value.as_ref()).await?;
        AsyncFramedWriter::dispose_msg(msg).await?;

        Ok(recv)
    }

    pub async fn send<T>(&self, value: &T) -> Result<quinn::RecvStream, RpcClientError>
    where
        T: Serialize<AllocSerializer<512>>,
    {
        let mut serializer = AllocSerializer::default();
        if let Err(e) = serializer.serialize_value(value) {
            log::error!("Error serializing RPC message: {e}");
            return Err(RpcClientError::EncodingError);
        }

        self.send_raw(serializer.into_serializer().into_inner()).await
    }
}

impl RpcClient {}
