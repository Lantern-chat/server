use std::ops::Deref;

use tokio_postgres::{
    types::{ToSql, Type},
    Client as DbClient, Error, Row, RowStream, Statement,
};

use crate::db::queries::{CachedQuery, PreparedQueryCache};

pub struct Client {
    client: DbClient,
    cache: PreparedQueryCache,
}

impl Client {
    pub async fn new(db: DbClient) -> Result<Self, Error> {
        let cache: PreparedQueryCache = PreparedQueryCache::populate(&db).await?;

        Ok(Client { client: db, cache })
    }
}

impl Deref for Client {
    type Target = DbClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl Client {
    pub async fn query_cached(
        &self,
        query: CachedQuery,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, Error> {
        self.query(self.cache.get(query), params).await
    }

    pub async fn query_one_cached(
        &self,
        query: CachedQuery,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Row, Error> {
        self.query_one(self.cache.get(query), params).await
    }
}
