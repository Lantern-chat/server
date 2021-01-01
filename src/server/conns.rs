use std::hash::{BuildHasher, Hash, Hasher};

use ahash::{AHasher, RandomState};
use hashbrown::HashMap;
use tokio::sync::{Mutex, RwLock};

use crate::{db::Snowflake, server::gateway::conn::ClientConnection, util::cmap::CHashMap};

#[derive(Default)]
pub struct HostConnections {
    pub conns: CHashMap<Snowflake, RwLock<Vec<ClientConnection>>>,
}
