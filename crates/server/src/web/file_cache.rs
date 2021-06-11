use std::borrow::Cow;
use std::io;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
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
    #[inline]
    fn is_dir(&self) -> bool {
        self.is_dir
    }
    #[inline]
    fn len(&self) -> u64 {
        self.len
    }
    #[inline]
    fn modified(&self) -> io::Result<SystemTime> {
        Ok(self.last_modified)
    }
    #[inline]
    fn blksize(&self) -> u64 {
        1024 * 8
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
    #[inline]
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let pos = self.pos as usize;
        let end = (pos + buf.remaining()).min(self.buf.len());

        buf.put_slice(&self.buf[pos..end]);

        self.pos = end as u64;

        Poll::Ready(Ok(()))
    }
}

impl AsyncSeek for CachedFile {
    #[inline]
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

    #[inline]
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
use util::cmap::EntryValue;

#[async_trait::async_trait]
impl FileCache for MainFileCache {
    type File = CachedFile;
    type Meta = Metadata;

    async fn open(
        &self,
        path: &Path,
        accepts: Option<AcceptEncoding>,
    ) -> io::Result<(Self::File, ContentCoding)> {
        let mut last_modified = None;

        match self.map.get_cloned(path).await {
            Some(file) => {
                let dur = SystemTime::now().duration_since(file.last_checked);

                match dur {
                    Err(_) => {
                        log::warn!("Duration calculation failed, time reversed?");
                    }
                    Ok(dur) if dur > Duration::from_secs(15) => {
                        last_modified = Some(file.last_modified);
                    }
                    Ok(_) => {
                        let coding = match accepts.and_then(|a| a.prefered_encoding()) {
                            None | Some(ContentCoding::COMPRESS) | Some(ContentCoding::IDENTITY) => {
                                ContentCoding::IDENTITY
                            }
                            Some(coding) => coding,
                        };

                        let file = CachedFile {
                            pos: 0,
                            last_modified: file.last_modified,
                            buf: match coding {
                                ContentCoding::BROTLI => file.brotli.clone(),
                                ContentCoding::DEFLATE => file.deflate.clone(),
                                ContentCoding::GZIP => file.gzip.clone(),
                                ContentCoding::IDENTITY => file.iden.clone(),
                                ContentCoding::COMPRESS => unreachable!(),
                            },
                        };

                        log::trace!(
                            "Serving cached {:?} ({}) encoded file: {}",
                            coding,
                            file.buf.len(),
                            path.display()
                        );

                        return Ok((file, coding));
                    }
                }
            }
            None => {}
        }

        use tokio::io::AsyncReadExt;

        // WARNING: This will lock an entire shard while the file is processed,
        // avoiding duplicate processing.
        let EntryValue { lock, mut entry } = self.map.entry(path).await;

        log::trace!("Loading in file to cache: {}", path.display());
        let mut file = tokio::fs::File::open(path).await?;

        let meta = file.metadata().await?;

        if !meta.is_file() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Not found"));
        }

        let mut do_read = true;

        if let Some(last_modified) = last_modified {
            log::trace!("Checking for modifications of {}", path.display());

            if last_modified == meta.modified()? {
                use hashbrown::hash_map::RawEntryMut;

                match entry {
                    RawEntryMut::Occupied(ref mut occupied) => {
                        occupied.get_mut().last_checked = SystemTime::now();

                        do_read = false;

                        log::trace!("NOT CHANGED");
                    }
                    _ => {}
                }
            }
        }

        if do_read {
            let mut content = Vec::new();
            file.read_to_end(&mut content).await?;

            let (brotli, deflate, gzip) = {
                use async_compression::{
                    tokio::bufread::{BrotliEncoder, DeflateEncoder, GzipEncoder},
                    Level,
                };

                let (level, brotli_level) = if cfg!(debug_assertions) {
                    (Level::Fastest, Level::Fastest)
                } else {
                    (Level::Best, Level::Precise(3))
                };

                let mut brotli_buffer = Vec::new();
                let mut deflate_buffer = Vec::new();
                let mut gzip_buffer = Vec::new();

                let mut brotli = BrotliEncoder::with_quality(&content[..], brotli_level);
                let mut deflate = DeflateEncoder::with_quality(&content[..], level);
                let mut gzip = GzipEncoder::with_quality(&content[..], level);

                let brotli = brotli.read_to_end(&mut brotli_buffer);
                let deflate = deflate.read_to_end(&mut deflate_buffer);
                let gzip = gzip.read_to_end(&mut gzip_buffer);

                let res = tokio::try_join! {
                    async { log::trace!("Compressing with Brotli"); brotli.await },
                    async { log::trace!("Compressing with Deflate"); deflate.await },
                    async { log::trace!("Compressing with GZip"); gzip.await }
                };

                res?;

                log::trace!(
                    "Brotli: {}, Deflate: {}, Gzip: {}",
                    brotli_buffer.len(),
                    deflate_buffer.len(),
                    gzip_buffer.len()
                );

                (brotli_buffer, deflate_buffer, gzip_buffer)
            };

            log::trace!("Inserting into cache");

            entry.insert(
                path.to_path_buf(),
                CacheEntry {
                    iden: Arc::from(content),
                    brotli: Arc::from(brotli),
                    deflate: Arc::from(deflate),
                    gzip: Arc::from(gzip),
                    last_modified: meta.modified()?,
                    last_checked: SystemTime::now(),
                },
            );
        }

        // release lock
        drop(lock);

        self.open(path, accepts).await
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
