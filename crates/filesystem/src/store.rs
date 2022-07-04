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

pub trait AsyncRWSeekStream: AsyncWrite + AsyncRead + AsyncSeek + FileExt + Send + Sync {}
impl<T> AsyncRWSeekStream for T where T: AsyncWrite + AsyncRead + AsyncSeek + FileExt + Send + Sync {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct FileStore<'a> {
    pub root: &'a Path,
}

impl FileStore<'_> {
    pub async fn open_crypt(
        &self,
        id: Snowflake,
        mode: OpenMode,
        options: &CipherOptions,
    ) -> io::Result<Pin<Box<dyn AsyncRWSeekStream>>> {
        let cipher = options.create();
        let file = self.open(id, mode).await?;

        Ok(match mode {
            OpenMode::Read => Box::pin(EncryptedFile::new(file, cipher)),
            // write-mode has some extra optimizations for buffering encrypted writes
            OpenMode::Write => Box::pin(EncryptedFile::new_write(file, cipher)),
        })
    }

    pub async fn open(&self, id: Snowflake, mode: OpenMode) -> io::Result<TkFile> {
        let mut path = self.root.to_path_buf();
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

        log::trace!(
            "Asynchronously opening file: {} in mode: {mode:?}",
            path.display()
        );

        options.open(path).await
    }

    pub async fn metadata(&self, id: Snowflake) -> io::Result<Metadata> {
        let mut path = self.root.to_path_buf();
        id_to_path(id, &mut path);
        id_to_name(id, &mut path);

        log::trace!("Loading metadata: {}", path.display());

        tokio::fs::metadata(path).await
    }
}

pub trait ReadSeekStream: io::Read + io::Seek {}
impl<T> ReadSeekStream for T where T: io::Read + io::Seek {}

pub trait WriteSeekStream: io::Write + io::Seek {}
impl<T> WriteSeekStream for T where T: io::Write + io::Seek {}

impl FileStore<'_> {
    pub fn open_crypt_write_sync(
        &self,
        id: Snowflake,
        options: &CipherOptions,
    ) -> io::Result<Box<dyn WriteSeekStream>> {
        let cipher = options.create();
        let file = self.open_sync(id, OpenMode::Write)?;

        Ok(Box::new(EncryptedFile::new_write_sync(file, cipher)))
    }

    pub fn open_crypt_read_sync(
        &self,
        id: Snowflake,
        options: &CipherOptions,
    ) -> io::Result<Box<dyn ReadSeekStream>> {
        let cipher = options.create();
        let file = self.open_sync(id, OpenMode::Read)?;

        Ok(Box::new(EncryptedFile::new(file, cipher)))
    }

    pub fn open_sync(&self, id: Snowflake, mode: OpenMode) -> io::Result<std::fs::File> {
        let mut path = self.root.to_path_buf();
        id_to_path(id, &mut path);

        if mode == OpenMode::Write {
            std::fs::create_dir_all(&path)?;
        }

        id_to_name(id, &mut path);

        let mut options = std::fs::OpenOptions::new();
        let _ = match mode {
            OpenMode::Read => options.read(true),
            OpenMode::Write => options.write(true).create(true),
        };

        log::trace!("Synchronously opening file: {} in mode: {mode:?}", path.display());

        options.open(path)
    }
}
