use crate::db::Snowflake;

use tokio::io::{self, AsyncRead, AsyncSeek, AsyncWrite};

pub mod fs;

pub trait AsyncSeekRead: AsyncRead + AsyncSeek {}
impl<T> AsyncSeekRead for T where T: AsyncRead + AsyncSeek {}

#[async_trait::async_trait]
pub trait FileStore {
    async fn writer(&self, file_id: Snowflake) -> Result<Box<dyn AsyncWrite>, anyhow::Error>;
    async fn reader(&self, file_id: Snowflake) -> Result<Box<dyn AsyncSeekRead>, anyhow::Error>;
    async fn delete(&self, file_id: Snowflake) -> Result<(), anyhow::Error>;
}
