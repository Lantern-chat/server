use hashbrown::HashMap;
use std::future::Future;
use tokio::sync::{RwLock, RwLockWriteGuard};
use tokio_postgres::{types::ToSql, Client, Error, Row, Statement, ToStatement};

use super::*;

pub struct PreparedQueryCache {
    pub cache: RwLock<HashMap<&'static str, Statement>>,
}

// Workaround for: `hidden type `impl futures::Future` captures lifetime smaller than the function body`
type StaticStr = &'static str;

#[async_trait::async_trait]
pub trait ClientExt {
    async fn query_cached(
        &self,
        cache: &PreparedQueryCache,
        query: &StaticStr,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, Error>;
}

#[async_trait::async_trait]
impl ClientExt for Client {
    async fn query_cached(
        &self,
        cache: &PreparedQueryCache,
        query: &StaticStr,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, Error> {
        let cache_read = cache.cache.read().await;

        let cached_stmt = cache_read.get(query).cloned();

        drop(cache_read);

        match cached_stmt {
            Some(stmt) => self.query(&stmt, params).await,
            None => {
                let stmt = self.prepare(query).await?;

                let cache_write_fut = cache.cache.write();
                let run_query_fut = self.query(&stmt, params);

                let (mut cache_write, res) = futures::join!(cache_write_fut, run_query_fut);

                cache_write.insert(query, stmt);

                res
            }
        }
    }
}
