use std::{
    any::{Any, TypeId},
    ops::Deref,
};

use parking_lot::RwLock;

use hashbrown::HashMap;
use tokio_postgres::{
    types::{ToSql, Type},
    Client as DbClient, Error, Row, RowStream, Statement,
};

pub struct Client {
    client: DbClient,
    cache: RwLock<HashMap<TypeId, Statement>>,
}

impl Client {
    pub fn new(db: DbClient) -> Self {
        Client {
            client: db,
            cache: Default::default(),
        }
    }
}

impl Deref for Client {
    type Target = DbClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl Client {
    pub async fn prepare_cached<F>(&self, query: F) -> Result<Statement, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        let id = TypeId::of::<F>();

        // NOTE: This uses a `parking_lot` RwLock here to take the fast path in acquiring the lock
        // without having to yield at all, since the cache is only populated at startup any write locks
        // will happen early-on in the program, leaving only nice fast-paths once warmed up.
        let cache = self.cache.upgradable_read();

        if let Some(stmt) = cache.get(&id) {
            return Ok(stmt.clone());
        }

        let stmt = self.prepare(query()).await?;

        let mut cache = parking_lot::RwLockUpgradableReadGuard::upgrade(cache);

        cache.insert(id, stmt.clone());

        Ok(stmt)
    }

    pub async fn query_cached<F>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query(&self.prepare_cached(query).await?, params).await
    }

    pub async fn query_one_cached<F>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query_one(&self.prepare_cached(query).await?, params)
            .await
    }

    pub async fn query_opt_cached<F>(
        &self,
        query: F,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error>
    where
        F: Any + FnOnce() -> &'static str,
    {
        self.query_opt(&self.prepare_cached(query).await?, params)
            .await
    }
}
