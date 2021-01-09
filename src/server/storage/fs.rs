use super::*;

use tokio::fs;

pub struct FilesystemFileStore {}

#[async_trait::async_trait]
impl FileStore for FilesystemFileStore {
    async fn writer(&self, file_id: Snowflake) -> Result<Box<dyn AsyncWrite>, anyhow::Error> {
        Ok(Box::new(
            fs::OpenOptions::new()
                .write(true)
                .open(format!("{}", file_id))
                .await?,
        ))
    }

    async fn reader(&self, file_id: Snowflake) -> Result<Box<dyn AsyncSeekRead>, anyhow::Error> {
        Ok(Box::new(
            fs::OpenOptions::new()
                .read(true)
                .open(format!("{}", file_id))
                .await?,
        ))
    }

    async fn delete(&self, file_id: Snowflake) -> Result<(), anyhow::Error> {
        fs::remove_file(format!("{}", file_id)).await?;
        Ok(())
    }
}
