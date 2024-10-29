use std::fs::Metadata;
use std::io;
use std::path::PathBuf;

use aes::cipher::{Iv, Key, KeyIvInit};
use aes::Aes256;

pub type Aes256Ctr = ctr::Ctr64BE<Aes256>;

use tokio::fs::{self, File as TkFile, OpenOptions};

use sdk::Snowflake;

use crate::path::{id_to_name, id_to_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    Read,
    Write,
}

pub struct CipherOptions {
    pub key: Key<Aes256>,
    pub nonce: Iv<Aes256Ctr>,
}

impl CipherOptions {
    pub fn create(&self) -> Aes256Ctr {
        Aes256Ctr::new(&self.key, &self.nonce)
    }

    #[inline]
    pub fn new_from_i64_nonce(key: Key<Aes256>, nonce: i64) -> Self {
        CipherOptions {
            key,
            nonce: unsafe { std::mem::transmute([nonce, nonce]) },
        }
    }
}

use super::crypt::EncryptedFile;

pub trait FileExt {
    fn set_len(&self, size: u64) -> impl std::future::Future<Output = Result<(), io::Error>> + Send;

    fn get_len(&self) -> impl std::future::Future<Output = Result<u64, io::Error>> + Send;
}

impl FileExt for TkFile {
    async fn set_len(&self, size: u64) -> Result<(), io::Error> {
        TkFile::set_len(self, size).await
    }

    async fn get_len(&self) -> Result<u64, io::Error> {
        let meta = self.metadata().await?;

        Ok(meta.len())
    }
}

impl<F: FileExt + Sync> FileExt for EncryptedFile<F> {
    async fn set_len(&self, size: u64) -> Result<(), io::Error> {
        self.get_ref().set_len(size).await
    }

    async fn get_len(&self) -> Result<u64, io::Error> {
        self.get_ref().get_len().await
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct FileStore {
    pub root: PathBuf,
}

impl FileStore {
    pub async fn delete(self, id: Snowflake) -> io::Result<()> {
        let mut path = self.root;
        id_to_path(id, &mut path);
        id_to_name(id, &mut path);
        fs::remove_file(path).await
    }

    pub async fn open_crypt(
        self,
        id: Snowflake,
        mode: OpenMode,
        options: &CipherOptions,
    ) -> io::Result<EncryptedFile<TkFile>> {
        let cipher = options.create();
        let file = self.open(id, mode).await?;

        Ok(EncryptedFile::new(file, cipher))
    }

    pub async fn open(self, id: Snowflake, mode: OpenMode) -> io::Result<TkFile> {
        let mut path = self.root;
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
            "Asynchronously opening file: {} ({id}) in mode: {mode:?}",
            path.display()
        );

        options.open(path).await
    }

    pub async fn metadata(self, id: Snowflake) -> io::Result<Metadata> {
        let mut path = self.root;
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

impl FileStore {
    pub fn open_crypt_write_sync(
        self,
        id: Snowflake,
        options: &CipherOptions,
    ) -> io::Result<EncryptedFile<std::fs::File>> {
        let cipher = options.create();
        let file = self.open_sync(id, OpenMode::Write)?;

        Ok(EncryptedFile::new(file, cipher))
    }

    pub fn open_crypt_read_sync(
        self,
        id: Snowflake,
        options: &CipherOptions,
    ) -> io::Result<EncryptedFile<std::fs::File>> {
        let cipher = options.create();
        let file = self.open_sync(id, OpenMode::Read)?;

        Ok(EncryptedFile::new(file, cipher))
    }

    pub fn open_sync(self, id: Snowflake, mode: OpenMode) -> io::Result<std::fs::File> {
        let mut path = self.root;
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

        log::trace!(
            "Synchronously opening file: {} ({id}) in mode: {mode:?}",
            path.display()
        );

        options.open(path)
    }
}
