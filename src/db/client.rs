use std::{
    any::{Any, TypeId},
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use futures::{Stream, StreamExt, TryStreamExt};
use hashbrown::HashMap;
use parking_lot::RwLock;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};
use tokio_postgres as pg;
use tokio_postgres::{
    types::{BorrowToSql, ToSql, Type},
    Row, RowStream, Statement, ToStatement,
};

use failsafe::futures::CircuitBreaker;
use failsafe::Config;

use arc_swap::{ArcSwap, ArcSwapOption};

use super::conn::ConnectionStream;

pub struct ClientInner {
    pub autoreconnect: AtomicBool,
    pub config: pg::Config,
    pub client: ArcSwapOption<pg::Client>,
    pub cache: ArcSwap<HashMap<TypeId, Statement>>,
    pub conn: Mutex<Option<UnboundedReceiver<pg::AsyncMessage>>>,
}

#[derive(Clone)]
pub struct Client(Arc<ClientInner>);

impl Deref for Client {
    type Target = ClientInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    #[error("Database error: {0}")]
    Db(#[from] pg::Error),
    #[error("Database is disconnected")]
    Disconnected,
}

impl Drop for ClientInner {
    fn drop(&mut self) {
        self.autoreconnect.store(false, Ordering::SeqCst);
    }
}

impl Client {
    fn spawn_forward<S, T>(
        this: Self,
        tx: UnboundedSender<pg::AsyncMessage>,
        connection: pg::Connection<S, T>,
    ) where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
        T: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        tokio::spawn(async move {
            let mut conn = ConnectionStream(connection);
            while let Some(msg) = conn.next().await {
                match msg {
                    Ok(msg) => {
                        if let Err(e) = tx.send(msg) {
                            log::error!("Error forwarding database event: {:?}", e);
                        }
                    }
                    Err(e) => {
                        log::error!("Database error: {:?}", e);
                        break;
                    }
                }
            }

            // dropping tx before acquiring the lock below will ensure the rx loops will break
            // and unlock the mutex
            drop(tx);

            this.client.store(None);
            *this.conn.lock().await = None;

            log::info!(
                "Disconnected from database {:?}",
                this.config.get_dbname().unwrap_or("Unnamed")
            );

            if this.autoreconnect.load(Ordering::SeqCst) {
                log::info!("Attempting reconnect...");

                if let Err(e) = this.reconnect().await {
                    log::error!("Reconnect error: {}", e);
                }
            }
        });
    }

    async fn real_connect(&self, attempt: u64) -> Result<(), anyhow::Error> {
        let name = self.config.get_dbname().unwrap_or("Unnamed");
        log::info!(
            "Connecting ({}) to database {:?} at {:?}:{:?}...",
            attempt,
            name,
            self.config.get_hosts(),
            self.config.get_ports(),
        );
        let (client, connection) = self.config.connect(pg::NoTls).await?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        Self::spawn_forward(self.clone(), tx, connection);

        *self.conn.lock().await = Some(rx);
        self.cache.store(Default::default());
        self.client.store(Some(Arc::new(client)));

        log::info!("Connection to database {:?} successful!", name);

        Ok(())
    }

    pub async fn reconnect(&self) -> Result<(), anyhow::Error> {
        self.client.store(None);

        let circuit_breaker = Config::new().build();

        for i in 1u64.. {
            match circuit_breaker.call(self.real_connect(i)).await {
                Ok(_) => return Ok(()),
                Err(failsafe::Error::Inner(e)) => {
                    log::error!("Connect error: {:?}", e);
                }
                Err(failsafe::Error::Rejected) => {
                    log::warn!("Connect rate-limited!");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }

        unreachable!()
    }

    pub async fn connect(config: pg::Config) -> Result<Self, anyhow::Error> {
        let this = Client(Arc::new(ClientInner {
            config,
            client: ArcSwapOption::from(None),
            cache: ArcSwap::default(),
            autoreconnect: AtomicBool::new(true),
            conn: Mutex::new(None),
        }));

        this.reconnect().await?;

        Ok(this)
    }
}

impl Client {
    pub fn client(&self) -> Result<Arc<pg::Client>, ClientError> {
        match self.client.load_full() {
            Some(client) => Ok(client),
            None => Err(ClientError::Disconnected),
        }
    }

    pub async fn close(&self) {
        self.autoreconnect.store(false, Ordering::SeqCst);
        self.client.store(None);
        *self.conn.lock().await = None;
    }

    pub async fn prepare_cached<F>(&self, query: F) -> Result<Statement, ClientError>
    where
        F: Any + FnOnce() -> &'static str,
    {
        let id = TypeId::of::<F>();

        // It's fine to get a cached entry if the client is disconnected
        // since it can't be used anyway.
        if let Some(stmt) = self.cache.load().get(&id) {
            return Ok(stmt.clone());
        }

        let stmt = self.client()?.prepare(query()).await?;

        self.cache.rcu(|cache| {
            let mut cache = HashMap::clone(&cache);
            cache.insert(id, stmt.clone());
            cache
        });

        Ok(stmt)
    }

    pub async fn query_raw<T, P, I>(
        &self,
        statement: &T,
        params: I,
    ) -> Result<RowStream, ClientError>
    where
        T: ?Sized + ToStatement,
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        self.client()?
            .query_raw(statement, params)
            .await
            .map_err(ClientError::from)
    }

    pub async fn query_raw_cached<F, P, I>(
        &self,
        query: F,
        params: I,
    ) -> Result<RowStream, ClientError>
    where
        F: Any + FnOnce() -> &'static str,
        P: BorrowToSql,
        I: IntoIterator<Item = P>,
        I::IntoIter: ExactSizeIterator,
    {
        self.query_raw(&self.prepare_cached(query).await?, params)
            .await
    }

    pub async fn query_stream<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<impl Stream<Item = Result<Row, ClientError>>, ClientError>
    where
        T: ?Sized + ToStatement,
    {
        fn slice_iter<'a>(
            s: &'a [&'a (dyn ToSql + Sync)],
        ) -> impl ExactSizeIterator<Item = &'a dyn ToSql> + 'a {
            s.iter().map(|s| *s as _)
        }

        Ok(self
            .query_raw(statement, slice_iter(params))
            .await?
            .map_err(ClientError::from))
    }

    pub async fn query_stream_cached<F>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<impl Stream<Item = Result<Row, ClientError>>, ClientError>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query_stream(&self.prepare_cached(query).await?, params)
            .await
    }

    pub async fn execute<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<u64, ClientError>
    where
        T: ?Sized + ToStatement,
    {
        self.client()?
            .execute(statement, params)
            .await
            .map_err(ClientError::from)
    }

    pub async fn execute_cached<F>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<u64, ClientError>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.client()?
            .execute(&self.prepare_cached(query).await?, params)
            .await
            .map_err(ClientError::from)
    }

    pub async fn query<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, ClientError>
    where
        T: ?Sized + ToStatement,
    {
        self.client()?
            .query(statement, params)
            .await
            .map_err(ClientError::from)
    }

    pub async fn query_cached<F>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, ClientError>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query(&self.prepare_cached(query).await?, params).await
    }

    pub async fn query_one<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, ClientError>
    where
        T: ?Sized + ToStatement,
    {
        self.client()?
            .query_one(statement, params)
            .await
            .map_err(ClientError::from)
    }

    pub async fn query_one_cached<F>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, ClientError>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query_one(&self.prepare_cached(query).await?, params)
            .await
    }

    pub async fn query_opt<T>(
        &self,
        statement: &T,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, ClientError>
    where
        T: ?Sized + ToStatement,
    {
        self.client()?
            .query_opt(statement, params)
            .await
            .map_err(ClientError::from)
    }

    pub async fn query_opt_cached<F>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, ClientError>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query_opt(&self.prepare_cached(query).await?, params)
            .await
    }
}
