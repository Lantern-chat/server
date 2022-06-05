use std::fs::Metadata;
use std::io;
use std::path::Path;
use std::pin::Pin;

use aes::cipher::{BlockCipherKey, NewCipher, Nonce};
use aes::{Aes256, Aes256Ctr};

use tokio::fs::{self, File as TkFile, OpenOptions};
use tokio::io::{AsyncRead, AsyncSeek, AsyncWrite, BufWriter};

use sdk::Snowflake;

use crate::path::{id_to_name, id_to_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    Read,
    Write,
}

pub struct CipherOptions {
    pub key: BlockCipherKey<Aes256>,
    pub nonce: Nonce<Aes256Ctr>,
}

impl CipherOptions {
    pub fn create(&self) -> Aes256Ctr {
        Aes256Ctr::new(&self.key, &self.nonce)
    }
}

use super::crypt::EncryptedFile;

#[async_trait::async_trait]
pub trait FileExt {
    async fn set_len(&self, size: u64) -> Result<(), io::Error>;

    async fn get_len(&self) -> Result<u64, io::Error>;
}

#[async_trait::async_trait]
impl FileExt for TkFile {
    async fn set_len(&self, size: u64) -> Result<(), io::Error> {
        TkFile::set_len(self, size).await
    }

    async fn get_len(&self) -> Result<u64, io::Error> {
        let meta = self.metadata().await?;

        Ok(meta.len())
    }
}

#[async_trait::async_trait]
impl FileExt for BufWriter<TkFile> {
    async fn set_len(&self, size: u64) -> Result<(), io::Error> {
        self.get_ref().set_len(size).await
    }

    async fn get_len(&self) -> Result<u64, io::Error> {
        self.get_ref().get_len().await
    }
}

#[async_trait::async_trait]
impl<F: FileExt + Sync> FileExt for EncryptedFile<F> {
    async fn set_len(&self, size: u64) -> Result<(), io::Error> {
        self.get_ref().set_len(size).await
    }

    async fn get_len(&self) -> Result<u64, io::Error> {
        self.get_ref().get_len().await
    }
}

pub trait RWSeekStream: AsyncWrite + AsyncRead + AsyncSeek + FileExt + Send + Sync {}
impl<T> RWSeekStream for T where T: AsyncWrite + AsyncRead + AsyncSeek + FileExt + Send + Sync {}

pub struct FileStore;

impl FileStore {
    pub async fn open_crypt(
        root: &Path,
        id: Snowflake,
        mode: OpenMode,
        options: &CipherOptions,
    ) -> io::Result<Pin<Box<dyn RWSeekStream>>> {
        let cipher = options.create();
        let file = FileStore::open(root, id, mode).await?;

        Ok(match mode {
            OpenMode::Read => Box::pin(EncryptedFile::new(file, cipher)),
            // write-mode has some extra optimizations for buffering encrypted writes
            OpenMode::Write => Box::pin(EncryptedFile::new_write(file, cipher)),
        })
    }

    pub async fn open(root: &Path, id: Snowflake, mode: OpenMode) -> io::Result<TkFile> {
        let mut path = root.to_path_buf();
        id_to_path(id, &mut path);

        if mode == OpenMode::Write {
            fs::create_dir_all(&path).await?;
        }

        id_to_name(id, &mut path);

        let mut options = OpenOptions::new();
        let _ = match mode {
            OpenMode::Read => options.read(true),
            OpenMode::Write => options.write(true).create(true),
        };

        log::trace!("Opening file: {} in mode: {mode:?}", path.display());

        options.open(path).await
    }

    pub async fn metadata(root: &Path, id: Snowflake) -> io::Result<Metadata> {
        let mut path = root.to_path_buf();
        id_to_path(id, &mut path);
        id_to_name(id, &mut path);

        log::trace!("Loading metadata: {}", path.display());

        tokio::fs::metadata(path).await
    }
}