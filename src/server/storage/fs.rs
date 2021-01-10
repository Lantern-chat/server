use std::path::PathBuf;

use super::*;

use tokio::fs;

pub struct FilesystemFileStore {
    pub base_path: PathBuf,
}

impl FilesystemFileStore {
    pub fn to_path(&self, file_id: Snowflake) -> PathBuf {
        let mut path = self.base_path.clone();
        path.push(file_id.to_string());
        path
    }
}

#[async_trait::async_trait]
impl FileStore for FilesystemFileStore {
    async fn writer(&self, file_id: Snowflake) -> Result<Box<dyn AsyncWrite>, anyhow::Error> {
        Ok(Box::new(
            fs::OpenOptions::new()
                .write(true)
                .open(self.to_path(file_id))
                .await?,
        ))
    }

    async fn reader(&self, file_id: Snowflake) -> Result<Box<dyn AsyncSeekRead>, anyhow::Error> {
        Ok(Box::new(
            fs::OpenOptions::new()
                .read(true)
                .open(self.to_path(file_id))
                .await?,
        ))
    }

    async fn delete(&self, file_id: Snowflake) -> Result<(), anyhow::Error> {
        Ok(fs::remove_file(self.to_path(file_id)).await?)
    }
}
