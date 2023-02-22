use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::atomic::AtomicU64;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Weak,
};
use std::task::{Context, Poll};
use std::{any::TypeId, collections::VecDeque};
use std::{borrow::Cow, time::Duration};

use async_trait::async_trait;

use arc_swap::{ArcSwap, ArcSwapOption};
use hashbrown::HashMap;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    Notify, Semaphore, TryAcquireError,
};

use parking_lot::{Mutex, RwLock};

use futures::{Future, FutureExt, Stream, StreamExt, TryFutureExt, TryStreamExt};
use pg::{
    tls::MakeTlsConnect, tls::TlsConnect, types::Type, AsyncMessage, Client as PgClient, Config as PgConfig,
    Connection as PgConnection, Error as PgError, IsolationLevel, NoTls, Notification, Socket, Statement,
    Transaction as PgTransaction, TransactionBuilder as PgTransactionBuilder,
};

use failsafe::futures::CircuitBreaker;
use failsafe::Config;

async fn timeout<O, E>(duration: Option<Duration>, future: impl Future<Output = Result<O, E>>) -> Result<O, Error>
where
    Error: From<E>,
{
    Ok(match duration {
        Some(duration) => tokio::time::timeout(duration, future).await??,
        None => future.await?,
    })
}

pub mod config;
pub mod error;

pub use error::Error;

pub use config::{PoolConfig, Timeouts};

/// Simple wrapper type for `pg::Connection` that returns the actual message in the future
pub struct ConnectionStream<S, T>(pub PgConnection<S, T>);

impl<S, T> Deref for ConnectionStream<S, T> {
    type Target = PgConnection<S, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S, T> Stream for ConnectionStream<S, T>
where
    S: AsyncRead + AsyncWrite + Unpin,
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<AsyncMessage, PgError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_message(cx)
    }
}

fn ro(readonly: bool) -> &'static str {
    if readonly {
        "read-only"
    } else {
        "writable"
    }
}

use futures::stream::BoxStream;

#[derive(Clone)]
pub struct Connection {
    pub readonly: bool,
    pub id: u64,
    pub stream: Arc<tokio::sync::Mutex<BoxStream<'static, Result<AsyncMessage, PgError>>>>,
    pub release: Arc<Notify>,
}

impl Connection {
    pub fn spawn_notifications(&self, size: usize, name_hint: Option<String>) -> Receiver<Notification> {
        let (tx, rx) = mpsc::channel(size);

        let this = self.clone();

        let name_hint = name_hint.unwrap_or_else(|| "Unnamed".to_owned());

        tokio::spawn(async move {
            let mut stream = this.stream.lock().await;

            let released = loop {
                let item = tokio::select! {
                    biased;
                    item = stream.next() => { item }
                    _ = this.release.notified() => { break true; }
                };

                match item {
                    Some(Ok(msg)) => match msg {
                        AsyncMessage::Notification(notif) => {
                            use mpsc::error::SendTimeoutError;
                            match tx.send_timeout(notif, Duration::from_secs(3)).await {
                                Ok(_) => {}
                                Err(SendTimeoutError::Closed(n)) => {
                                    // other half has been closed, implying a drop, so exit early
                                    log::warn!("Failed to forward database notification: {:?}", n);
                                    break false;
                                }
                                Err(SendTimeoutError::Timeout(n)) => {
                                    log::error!("Forwarding database notification timed out: {:?}", n);
                                }
                            }
                        }
                        AsyncMessage::Notice(notice) => log::info!("Database notice: {notice}"),
                        _ => unreachable!("AsyncMessage is non-exhaustive"),
                    },
                    Some(Err(e)) => {
                        log::error!("Database connection error: {e}");
                        break false;
                    }
                    None => break false,
                }
            };

            drop(tx);

            if released {
                log::info!("Released {} connection loop to database {name_hint}", ro(this.readonly),);
            } else {
                log::info!("Disconnected from {} database {name_hint}", ro(this.readonly));
            }
        });

        rx
    }
}

#[async_trait]
pub trait Connector {
    async fn connect(&self, config: &PoolConfig) -> Result<(PgClient, Connection, Receiver<Notification>), Error>;
}

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[async_trait]
impl<T> Connector for T
where
    T: MakeTlsConnect<Socket> + Clone + Sync + Send + 'static,
    T::Stream: Sync + Send,
    T::TlsConnect: Sync + Send,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    async fn connect(&self, config: &PoolConfig) -> Result<(PgClient, Connection, Receiver<Notification>), Error> {
        let name = config.pg_config.get_dbname().unwrap_or("Unnamed");

        let circuit_breaker = Config::new().build();

        let mut attempt = 1;
        let (client, connection) = loop {
            // NOTE: This async block is not evaluated until polled, and when the circuitbreaker rejects
            // a future for rate-limiting, it is not polled, therefore this doesn't run on rejection.
            let connecting = async {
                log::info!(
                    "Connecting ({attempt}) to {} database {name} at {:?}:{:?}...",
                    ro(config.readonly),
                    config.pg_config.get_hosts(),
                    config.pg_config.get_ports(),
                );

                config.pg_config.connect(self.clone()).await
            };

            match circuit_breaker.call(connecting).await {
                Ok(res) => break res,
                Err(failsafe::Error::Inner(e)) => {
                    log::error!("Error connecting to database {name}: {e}");

                    attempt += 1;

                    if attempt > config.max_retries {
                        return Err(e.into());
                    }
                }
                Err(failsafe::Error::Rejected) => {
                    log::warn!("Connecting to database {name} rate-limited");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        };

        let conn = Connection {
            readonly: config.readonly,
            id: ID_COUNTER.fetch_add(1, Ordering::Relaxed),
            stream: Arc::new(tokio::sync::Mutex::new(ConnectionStream(connection).boxed())),
            release: Arc::new(Notify::new()),
        };

        let rx = conn.spawn_notifications(config.channel_size, Some(name.to_owned()));

        Ok((client, conn, rx))
    }
}

pub struct PoolInner {
    config: ArcSwap<PoolConfig>,
    connector: Box<dyn Connector + Send + Sync + 'static>,
    queue: Mutex<VecDeque<Client>>,
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
        C: Connector + Send + Sync + 'static,
    {
        Pool(Arc::new(PoolInner {
            semaphore: Semaphore::new(config.max_connections),
            connector: Box::new(conn),
            queue: Mutex::new(VecDeque::with_capacity(config.max_connections)),
            stmt_caches: StatementCaches::default(),
            config: ArcSwap::from_pointee(config),
        }))
    }

    pub fn replace_config(&self, config: PoolConfig) {
        if **self.config.load() != config {
            // avoid creating new connections while storing new config
            let mut queue = self.queue.lock();
            self.config.store(Arc::new(config));
            // TODO: Figure out how semaphore should be updated
            queue.clear();
        }
    }

    async fn create(&self) -> Result<Client, Error> {
        let config = self.config.load_full();

        let (client, conn, rx) = self.connector.connect(&config).await?;

        let stmt_cache = Arc::new(StatementCache::default());
        self.stmt_caches.attach(&stmt_cache);

        Ok(Client {
            readonly: config.readonly,
            config,
            client,
            rx,
            conn,
            stmt_cache,
        })
    }

    async fn recycle(&self, client: &Client) -> Result<(), Error> {
        if client.client.is_closed() {
            log::info!(
                "Connection {} could not be recycled because it was closed",
                client.conn.id
            );
            return Err(Error::RecyclingError);
        }

        if let Some(sql) = self.config.load().recycling_method.query() {
            if let Err(e) = client.client.simple_query(sql).await {
                log::warn!("Connection could not be recycled: {e}");
                return Err(Error::RecyclingError);
            }
        }

        Ok(())
    }

    pub async fn get(&self) -> Result<Object, Error> {
        self.timeout_get(&self.config.load().timeouts).await
    }

    pub async fn try_get(&self) -> Result<Object, Error> {
        let mut timeouts = self.config.load().timeouts.clone();
        timeouts.wait = Some(Duration::from_secs(0));
        self.timeout_get(&timeouts).await
    }

    pub async fn timeout_get(&self, timeouts: &Timeouts) -> Result<Object, Error> {
        let mut client = Object {
            inner: None,
            state: State::Waiting,
            pool: Arc::downgrade(&self.0),
        };

        let non_blocking = match timeouts.wait {
            Some(t) => t == Duration::from_nanos(0),
            None => false,
        };

        let permit = if non_blocking {
            self.semaphore.try_acquire().map_err(|e| match e {
                TryAcquireError::Closed => Error::Closed,
                TryAcquireError::NoPermits => Error::Timeout,
            })?
        } else {
            timeout(timeouts.wait, self.semaphore.acquire().map_err(|_| Error::Closed)).await?
        };

        permit.forget();

        loop {
            client.state = State::Receiving;

            let inner_client = {
                let mut queue = self.queue.lock();
                queue.pop_front()
            };

            match inner_client {
                Some(inner_client) => {
                    client.state = State::Recycling;
                    client.inner = Some(inner_client);

                    match timeout(timeouts.recycle, self.recycle(&client)).await {
                        Ok(_) => break,

                        // Note that in this case the `client` is reused
                        // The inner client is dropped next round by being overwritten
                        Err(_) => continue,
                    }
                }
                None => {
                    client.state = State::Creating;
                    client.inner = Some(timeout(timeouts.create, self.create()).await?);

                    break;
                }
            }
        }

        client.state = State::Ready;

        Ok(client)
    }

    pub async fn close(&self) {
        self.semaphore.close();
        self.queue.lock().clear();
    }
}

#[derive(Default)]
pub struct StatementCaches {
    caches: RwLock<Vec<Weak<StatementCache>>>,
}

impl StatementCaches {
    pub fn attach(&self, cache: &Arc<StatementCache>) {
        let cache = Arc::downgrade(cache);
        self.caches.write().push(cache);
    }

    pub fn detach(&self, cache: &Arc<StatementCache>) {
        let cache = Arc::downgrade(cache);
        self.caches.write().retain(|sc| !sc.ptr_eq(&cache));
    }

    pub fn clear(&self) {
        let caches = self.caches.read();
        for cache in caches.iter() {
            if let Some(cache) = cache.upgrade() {
                cache.clear();
            }
        }
    }

    pub fn cleanup(&self) {
        self.caches.write().retain(|sc| sc.strong_count() > 0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Waiting,
    Receiving,
    Creating,
    Recycling,
    Ready,
    Taken,
    Dropped,
}

pub struct Object {
    inner: Option<Client>,
    pool: Weak<PoolInner>,
    state: State,
}

impl Object {
    fn inner(&self) -> &Client {
        match self.inner {
            Some(ref inner) => inner,
            None => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    fn inner_mut(&mut self) -> &mut Client {
        match self.inner {
            Some(ref mut inner) => inner,
            None => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    pub fn take(mut this: Self) -> Client {
        this.state = State::Taken;
        if let Some(pool) = this.pool.upgrade() {
            pool.stmt_caches.detach(&this.stmt_cache);
        }
        this.inner.take().expect("Double-take of client")
    }
}

impl Deref for Object {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl DerefMut for Object {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner_mut()
    }
}

impl Drop for Object {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.upgrade() {
            match self.state {
                State::Receiving | State::Creating | State::Taken => pool.semaphore.add_permits(1),
                State::Recycling | State::Ready => {
                    let client = self.inner.take().expect("Double-take of dropped client");

                    // if still using the same config, allow reuse of this connection
                    if Arc::ptr_eq(&client.config, &pool.config.load()) {
                        let mut queue = pool.queue.lock();
                        queue.push_back(client);
                    }

                    // even if we didn't add this client back into the queue,
                    // it frees up space for a new connection
                    pool.semaphore.add_permits(1);
                }
                State::Waiting | State::Dropped => {}
            }
        }
        self.inner = None;
        self.state = State::Dropped;
    }
}

pub struct StatementCache {
    cache: ArcSwap<HashMap<TypeId, Statement>>,
}

impl Default for StatementCache {
    fn default() -> Self {
        StatementCache {
            cache: ArcSwap::new(Arc::new(HashMap::new())),
        }
    }
}

impl StatementCache {
    pub fn insert(&self, key: TypeId, stmt: &Statement) {
        self.cache.rcu(|cache| {
            let mut cache = HashMap::clone(cache);
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

pub struct Client {
    readonly: bool,
    client: PgClient,
    config: Arc<PoolConfig>,
    conn: Connection,
    rx: Receiver<Notification>,

    // NOTE: This is an Arc to allow cloning it to transactions without needing a ref
    pub stmt_cache: Arc<StatementCache>,
}

impl AsRef<PgClient> for Client {
    fn as_ref(&self) -> &PgClient {
        &self.client
    }
}

pub struct Transaction<'a> {
    t: PgTransaction<'a>,
    id: u64,
    stmt_cache: Arc<StatementCache>,
    readonly: bool,
}

impl Client {
    pub async fn take_connection(&self) -> Connection {
        self.conn.release.notify_one();
        drop(self.conn.stream.lock().await);
        self.conn.clone()
    }

    pub async fn recv_notif(&mut self) -> Option<Notification> {
        self.rx.recv().await
    }

    pub async fn transaction(&mut self) -> Result<Transaction, Error> {
        Ok(Transaction {
            readonly: self.readonly,
            id: self.conn.id,
            stmt_cache: self.stmt_cache.clone(),
            t: self.client.transaction().await?,
        })
    }
}

use std::any::Any;
use thorn::AnyQuery;

use pg::{
    types::{BorrowToSql, ToSql},
    Row, RowStream, ToStatement,
};

// TODO: I'm sure there is something better than a regex for this
lazy_static::lazy_static! {
    static ref WRITE_REGEX: regex::Regex =
        regex::RegexBuilder::new(r#"\b(UPDATE|INSERT|ALTER|CREATE|DROP|GRANT|REVOKE|DELETE|TRUNCATE)\b"#).build().unwrap();
}

impl Client {
    #[inline(always)]
    fn debug_check_readonly<'a>(&self, query: &'a str) -> &'a str {
        if cfg!(debug_assertions) && self.readonly {
            assert!(!WRITE_REGEX.is_match(query));
        }

        query
    }

    pub async fn prepare_cached<F>(&self, query: F) -> Result<Statement, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        let id = TypeId::of::<F>();

        // It's fine to get a cached entry if the client is disconnected
        // since it can't be used anyway.
        if let Some(stmt) = self.stmt_cache.get(id) {
            return Ok(stmt);
        }

        let stmt = self.client.prepare(self.debug_check_readonly(query())).boxed().await?;

        self.stmt_cache.insert(id, &stmt);

        Ok(stmt)
    }

    pub async fn query_raw<T, P, I>(&self, statement: &T, params: I) -> Result<RowStream, Error>
    where
        T: ?Sized + ToStatement,
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        self.client.query_raw(statement, params).await.map_err(Error::from)
    }

    pub async fn query_raw_cached<F, P, I>(&self, query: F, params: I) -> Result<RowStream, Error>
    where
        F: Any + FnOnce() -> &'static str,
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        self.query_raw(&self.prepare_cached(query).await?, params).await
    }

    pub async fn query_stream<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<impl Stream<Item = Result<Row, Error>>, Error>
    where
        T: ?Sized + ToStatement,
    {
        fn slice_iter<'a>(s: &'a [&'a (dyn ToSql + Sync)]) -> impl ExactSizeIterator<Item = &'a dyn ToSql> + 'a {
            s.iter().map(|s| *s as _)
        }

        Ok(self
            .query_raw(statement, slice_iter(params))
            .await?
            .map_err(Error::from))
    }

    pub async fn query_stream_cached<F>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<impl Stream<Item = Result<Row, Error>>, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query_stream(&self.prepare_cached(query).await?, params).await
    }

    pub async fn execute<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement,
    {
        self.client.execute(statement, params).await.map_err(Error::from)
    }

    pub async fn execute_cached<F>(&self, query: F, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.client
            .execute(&self.prepare_cached(query).await?, params)
            .await
            .map_err(Error::from)
    }

    pub async fn query<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error>
    where
        T: ?Sized + ToStatement,
    {
        self.client.query(statement, params).await.map_err(Error::from)
    }

    pub async fn query_cached<F>(&self, query: F, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query(&self.prepare_cached(query).await?, params).await
    }

    pub async fn query_one<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error>
    where
        T: ?Sized + ToStatement,
    {
        self.client.query_one(statement, params).await.map_err(Error::from)
    }

    pub async fn query_one_cached<F>(&self, query: F, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query_one(&self.prepare_cached(query).await?, params).await
    }

    pub async fn query_opt<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>, Error>
    where
        T: ?Sized + ToStatement,
    {
        self.client.query_opt(statement, params).await.map_err(Error::from)
    }

    pub async fn query_opt_cached<F>(&self, query: F, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query_opt(&self.prepare_cached(query).await?, params).await
    }
}

impl Client {
    pub async fn prepare_cached_typed<F, Q>(&self, query: F) -> Result<Statement, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        let id = TypeId::of::<F>();

        // It's fine to get a cached entry if the client is disconnected
        // since it can't be used anyway.
        if let Some(stmt) = self.stmt_cache.get(id) {
            return Ok(stmt);
        }

        let (query, collector) = query().to_string();
        let types = collector.types();

        log::debug!("Preparing {} query: \"{query}\"", self.conn.id);

        // this future is boxed to avoid extra growth on the stack of async functions calling this
        // and since it's rare for this to be reached (only on startup), the allocation cost is trivial
        let stmt = self
            .client
            .prepare_typed(self.debug_check_readonly(&query), &types)
            .boxed()
            .await?;

        self.stmt_cache.insert(id, &stmt);

        Ok(stmt)
    }

    pub async fn query_stream_cached_typed<F, Q>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<impl Stream<Item = Result<Row, Error>>, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        self.query_stream(&self.prepare_cached_typed(query).await?, params)
            .await
    }

    pub async fn execute_cached_typed<F, Q>(&self, query: F, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        self.execute(&self.prepare_cached_typed(query).await?, params).await
    }

    pub async fn query_cached_typed<F, Q>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        self.query(&self.prepare_cached_typed(query).await?, params).await
    }

    pub async fn query_one_cached_typed<F, Q>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        self.query_one(&self.prepare_cached_typed(query).await?, params).await
    }

    pub async fn query_opt_cached_typed<F, Q>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        self.query_opt(&self.prepare_cached_typed(query).await?, params).await
    }
}

impl Transaction<'_> {
    pub async fn commit(self) -> Result<(), Error> {
        self.t.commit().await.map_err(Error::from)
    }

    pub async fn rollback(self) -> Result<(), Error> {
        self.t.rollback().await.map_err(Error::from)
    }

    pub async fn transaction<'a>(&'a mut self) -> Result<Transaction<'a>, Error> {
        Ok(Transaction {
            readonly: self.readonly,
            id: self.id,
            stmt_cache: self.stmt_cache.clone(),
            t: self.t.transaction().await?,
        })
    }

    pub async fn savepoint<'a, I>(&'a mut self, name: I) -> Result<Transaction<'a>, Error>
    where
        I: Into<String>,
    {
        Ok(Transaction {
            readonly: self.readonly,
            id: self.id,
            stmt_cache: self.stmt_cache.clone(),
            t: self.t.savepoint(name).await?,
        })
    }

    pub fn cancel_token(&self) -> pg::CancelToken {
        self.t.cancel_token()
    }
}

impl Transaction<'_> {
    #[inline(always)]
    fn debug_check_readonly<'a>(&self, query: &'a str) -> &'a str {
        if cfg!(debug_assertions) && self.readonly {
            assert!(!WRITE_REGEX.is_match(query));
        }

        query
    }

    pub async fn prepare_cached<F>(&self, query: F) -> Result<Statement, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        let id = TypeId::of::<F>();

        // It's fine to get a cached entry if the client is disconnected
        // since it can't be used anyway.
        if let Some(stmt) = self.stmt_cache.get(id) {
            return Ok(stmt);
        }

        let stmt = self.t.prepare(self.debug_check_readonly(query())).await?;

        self.stmt_cache.insert(id, &stmt);

        Ok(stmt)
    }

    pub async fn query_raw<T, P, I>(&self, statement: &T, params: I) -> Result<RowStream, Error>
    where
        T: ?Sized + ToStatement,
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        self.t.query_raw(statement, params).await.map_err(Error::from)
    }

    pub async fn query_raw_cached<F, P, I>(&self, query: F, params: I) -> Result<RowStream, Error>
    where
        F: Any + FnOnce() -> &'static str,
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        self.query_raw(&self.prepare_cached(query).await?, params).await
    }

    pub async fn query_stream<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<impl Stream<Item = Result<Row, Error>>, Error>
    where
        T: ?Sized + ToStatement,
    {
        fn slice_iter<'a>(s: &'a [&'a (dyn ToSql + Sync)]) -> impl ExactSizeIterator<Item = &'a dyn ToSql> + 'a {
            s.iter().map(|s| *s as _)
        }

        Ok(self
            .query_raw(statement, slice_iter(params))
            .await?
            .map_err(Error::from))
    }

    pub async fn query_stream_cached<F>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<impl Stream<Item = Result<Row, Error>>, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query_stream(&self.prepare_cached(query).await?, params).await
    }

    pub async fn execute<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        T: ?Sized + ToStatement,
    {
        self.t.execute(statement, params).await.map_err(Error::from)
    }

    pub async fn execute_cached<F>(&self, query: F, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.t
            .execute(&self.prepare_cached(query).await?, params)
            .await
            .map_err(Error::from)
    }

    pub async fn query<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error>
    where
        T: ?Sized + ToStatement,
    {
        self.t.query(statement, params).await.map_err(Error::from)
    }

    pub async fn query_cached<F>(&self, query: F, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query(&self.prepare_cached(query).await?, params).await
    }

    pub async fn query_one<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error>
    where
        T: ?Sized + ToStatement,
    {
        self.t.query_one(statement, params).await.map_err(Error::from)
    }

    pub async fn query_one_cached<F>(&self, query: F, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query_one(&self.prepare_cached(query).await?, params).await
    }

    pub async fn query_opt<T>(&self, statement: &T, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>, Error>
    where
        T: ?Sized + ToStatement,
    {
        self.t.query_opt(statement, params).await.map_err(Error::from)
    }

    pub async fn query_opt_cached<F>(&self, query: F, params: &[&(dyn ToSql + Sync)]) -> Result<Option<Row>, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query_opt(&self.prepare_cached(query).await?, params).await
    }
}

impl Transaction<'_> {
    pub async fn prepare_cached_typed<F, Q>(&self, query: F) -> Result<Statement, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        let id = TypeId::of::<F>();

        // It's fine to get a cached entry if the client is disconnected
        // since it can't be used anyway.
        if let Some(stmt) = self.stmt_cache.get(id) {
            return Ok(stmt);
        }

        let (query, collector) = query().to_string();
        let types = collector.types();

        log::debug!("Preparing query: \"{query}\"");

        let stmt = self.t.prepare_typed(self.debug_check_readonly(&query), &types).await?;

        self.stmt_cache.insert(id, &stmt);

        Ok(stmt)
    }

    pub async fn query_stream_cached_typed<F, Q>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<impl Stream<Item = Result<Row, Error>>, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        self.query_stream(&self.prepare_cached_typed(query).await?, params)
            .await
    }

    pub async fn execute_cached_typed<F, Q>(&self, query: F, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        self.execute(&self.prepare_cached_typed(query).await?, params).await
    }

    pub async fn query_cached_typed<F, Q>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        self.query(&self.prepare_cached_typed(query).await?, params).await
    }

    pub async fn query_one_cached_typed<F, Q>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        self.query_one(&self.prepare_cached_typed(query).await?, params).await
    }

    pub async fn query_opt_cached_typed<F, Q>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error>
    where
        F: Any + FnOnce() -> Q,
        Q: AnyQuery,
    {
        self.query_opt(&self.prepare_cached_typed(query).await?, params).await
    }
}
