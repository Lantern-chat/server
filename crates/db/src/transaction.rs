use tokio_postgres as pg;
use tokio_postgres::{
    types::{BorrowToSql, ToSql, Type},
    Row, RowStream, Statement, ToStatement, Transaction as DbTransaction,
};

use crate::{client::Client, ClientError};

pub struct Transaction<'a> {
    t: DbTransaction<'a>,
    c: &'a mut Client,
}

impl<'a> Transaction<'a> {
    pub async fn commit(self) -> Result<(), ClientError> {
        self.t.commit().await.map_err(ClientError::from)
    }
}
