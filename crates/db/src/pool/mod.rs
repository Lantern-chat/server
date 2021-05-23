use std::borrow::Cow;
use std::ops::{Deref, DerefMut};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Weak,
};
use std::{any::TypeId, collections::VecDeque};

use async_trait::async_trait;

use arc_swap::{ArcSwap, ArcSwapOption};
use hashbrown::HashMap;

use tokio::sync::{
    mpsc::{self, error::RecvError, Receiver, Sender},
    Semaphore,
};

use parking_lot::{Mutex, RwLock};

use futures::{FutureExt, StreamExt};
use tokio_postgres::{
    tls::MakeTlsConnect, tls::TlsConnect, types::Type, AsyncMessage, Client as PgClient,
    Config as PgConfig, Error as PgError, IsolationLevel, NoTls, Notification, Socket, Statement,
    Transaction as PgTransaction, TransactionBuilder as PgTransactionBuilder,
};

pub mod config;

use config::PoolConfig;

use crate::conn::ConnectionStream;

#[async_trait]
pub trait Connector {
    async fn connect(
        &self,
        config: PoolConfig,
    ) -> Result<(PgClient, Receiver<Notification>), PgError>;
}

#[async_trait]
impl<T> Connector for T
where
    T: MakeTlsConnect<Socket> + Clone + Sync + Send + 'static,
    T::Stream: Sync + Send,
    T::TlsConnect: Sync + Send,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    async fn connect(
        &self,
        config: PoolConfig,
    ) -> Result<(PgClient, Receiver<Notification>), PgError> {
        let (client, connection) = config.pg_config.connect(self.clone()).await?;

        let (tx, rx) = mpsc::channel(config.channel_size);

        tokio::spawn(async move {
            let mut stream = ConnectionStream(connection);

            loop {
                match stream.next().await {
                    Some(Ok(msg)) => match msg {
                        AsyncMessage::Notification(notif) => {
                            if let Err(e) = tx.send(notif).await {
                                log::error!("Error forwarding database message: {}", e);
                                break;
                            }
                        }
                        AsyncMessage::Notice(notice) => log::info!("Database notice: {}", notice),
                        _ => {}
                    },
                    Some(Err(e)) => {
                        log::error!("Database connection error: {}", e);
                        break;
                    }
                    None => break,
                }
            }

            drop(tx);

            log::info!(
                "Disconnected from database {:?}",
                config.pg_config.get_dbname().unwrap_or("Unnamed")
            );
        });

        Ok((client, rx))
    }
}

pub struct PoolInner {
    config: PoolConfig,
    connector: Box<dyn Connector>,
    queue: Mutex<VecDeque<InnerClient>>,
    semaphore: Semaphore,

    pub stmt_caches: StatementCaches,
}

#[derive(Clone)]
pub struct Pool(Arc<PoolInner>);

impl Deref for Pool {
    type Target = PoolInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Pool {
    pub fn new<C>(config: PoolConfig, conn: C) -> Pool
    where
        C: Connector + 'static,
    {
        Pool(Arc::new(PoolInner {
            semaphore: Semaphore::new(config.max_connections),
            connector: Box::new(conn),
            queue: Default::default(),
            stmt_caches: StatementCaches::default(),
            config,
        }))
    }

    async fn recycle(&self, inner: &InnerClient) -> Result<(), PgError> {
        if inner.client.is_closed() {
            log::info!("Connection could not be recycled because it was closed");
        }

        if let Some(sql) = self.config.recycling_method.query() {
            if let Err(e) = inner.client.simple_query(sql).await {
                log::info!("Connection could not be recycled: {}", e);
            }
        }

        Ok(())
    }

    //pub async fn get(&self) -> Result<Client
}

#[derive(Default)]
pub struct StatementCaches {
    caches: RwLock<Vec<Weak<StatementCache>>>,
}

impl StatementCaches {
    pub async fn clear(&self) {
        let caches = self.caches.read();
        for cache in caches.iter() {
            if let Some(cache) = cache.upgrade() {
                cache.clear();
            }
        }
    }
}

pub struct StatementCache {
    cache: ArcSwap<HashMap<TypeId, Statement>>,
}

impl StatementCache {
    pub fn new() -> StatementCache {
        StatementCache {
            cache: ArcSwap::new(Arc::new(HashMap::new())),
        }
    }

    pub fn insert(&self, key: TypeId, stmt: &Statement) {
        self.cache.rcu(|cache| {
            let mut cache = HashMap::clone(&cache);
            cache.insert(key, stmt.clone());
            cache
        });
    }

    pub fn get(&self, key: TypeId) -> Option<Statement> {
        self.cache.load().get(&key).cloned()
    }

    pub fn clear(&self) {
        self.cache.store(Arc::new(HashMap::new()));
    }
}

enum State {
    Waiting,
    Receiving,
    Creating,
    Recycling,
    Ready,
    Taken,
    Dropped,
}

struct InnerClient {
    readonly: bool,
    client: PgClient,
    rx: Receiver<Notification>,
}

pub struct Client {
    inner: Option<InnerClient>,
    pool: Weak<PoolInner>,
    state: State,

    pub stmt_cache: Arc<StatementCache>,
}

impl Client {
    fn inner(&self) -> &InnerClient {
        match self.inner {
            Some(ref inner) => inner,
            None => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    fn inner_mut(&mut self) -> &mut InnerClient {
        match self.inner {
            Some(ref mut inner) => inner,
            None => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    pub async fn recv_notif(&mut self) -> Option<Notification> {
        self.inner_mut().rx.recv().await
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.upgrade() {
            match self.state {
                State::Waiting => {}
                State::Receiving | State::Creating | State::Taken => pool.semaphore.add_permits(1),
                State::Recycling | State::Ready => {
                    let client = self.inner.take().expect("Double-take of dropped client");
                    {
                        let mut queue = pool.queue.lock();
                        queue.push_back(client);
                    }

                    pool.semaphore.add_permits(1);
                }
                State::Dropped => {}
            }
        }
        self.inner = None;
        self.state = State::Dropped;
    }
}
