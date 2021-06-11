use std::borrow::Cow;
use std::io;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::SystemTime;

use tokio::io::AsyncSeek;
use tokio::io::{AsyncRead, ReadBuf};
use util::cmap::CHashMap;

use ftl::fs::{FileCache, FileMetadata, GenericFile};
use ftl::*;

use headers::ContentCoding;

pub struct Metadata {
    is_dir: bool,
    len: u64,
    last_modified: SystemTime,
}

impl FileMetadata for Metadata {
    fn is_dir(&self) -> bool {
        self.is_dir
    }
    fn len(&self) -> u64 {
        self.len
    }
    fn modified(&self) -> io::Result<SystemTime> {
        Ok(self.last_modified)
    }
    fn blksize(&self) -> u64 {
        self.len
    }
}

#[derive(Clone)]
pub struct CachedFile {
    buf: Arc<[u8]>,
    pos: u64,
    //encoding: ContentCoding,
    last_modified: SystemTime,
}

impl AsyncRead for CachedFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let pos = self.pos as usize;
        let end = (pos + buf.remaining()).min(self.buf.len() - 1);

        buf.put_slice(&self.buf[pos..end]);

        self.pos = end as u64;

        Poll::Ready(Ok(()))
    }
}

impl AsyncSeek for CachedFile {
    fn start_seek(mut self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
        self.pos = match position {
            SeekFrom::Current(offset) => (self.pos as i64).saturating_add(offset) as u64,
            SeekFrom::End(offset) => (self.buf.len() as i64).saturating_add(offset) as u64,
            SeekFrom::Start(offset) => offset,
        };

        if self.pos >= self.buf.len() as u64 {
            Err(io::Error::new(io::ErrorKind::Other, "Invalid seek!"))
        } else {
            Ok(())
        }
    }

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        Poll::Ready(Ok(self.pos))
    }
}

#[derive(Clone)]
pub struct CacheEntry {
    iden: Arc<[u8]>,
    brotli: Arc<[u8]>,
    gzip: Arc<[u8]>,
    deflate: Arc<[u8]>,
    last_modified: SystemTime,
    last_checked: SystemTime,
}

use hashbrown::HashMap;

#[derive(Default)]
pub struct MainFileCache {
    map: CHashMap<PathBuf, CacheEntry>,
}

use headers::AcceptEncoding;

#[async_trait::async_trait]
impl FileCache for MainFileCache {
    type File = CachedFile;
    type Meta = Metadata;

    async fn open(
        &self,
        path: &Path,
        accepts: Option<AcceptEncoding>,
    ) -> io::Result<(Self::File, ContentCoding)> {
        unimplemented!()
    }

    async fn metadata(&self, path: &Path) -> io::Result<Self::Meta> {
        match self.map.get(path).await {
            Some(file) => Ok(Metadata {
                len: file.iden.len() as u64,
                last_modified: file.last_modified,
                is_dir: false,
            }),
            None => {
                let meta = tokio::fs::metadata(path).await?;

                Ok(Metadata {
                    is_dir: meta.is_dir(),
                    len: meta.len(),
                    last_modified: meta.modified()?,
                })
            }
        }
    }

    async fn file_metadata(&self, file: &Self::File) -> io::Result<Self::Meta> {
        Ok(Metadata {
            len: file.buf.len() as u64,
            last_modified: file.last_modified,
            is_dir: false,
        })
    }
}
