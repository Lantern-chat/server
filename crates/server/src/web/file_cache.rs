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

use ftl::fs::{EncodedFile, FileCache, FileMetadata, GenericFile};
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
    encoding: ContentCoding,
    last_modified: SystemTime,
}

impl EncodedFile for CachedFile {
    fn encoding(&self) -> ContentCoding {
        self.encoding
    }
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
    best: ContentCoding,
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

impl MainFileCache {
    pub async fn cleanup(&self) {
        let now = SystemTime::now();

        self.map
            .retain(|_, file| match now.duration_since(file.last_checked) {
                // retain if duration since last checked is less than 1 hour (debug) or 24 hours (release)
                Ok(dur) => dur < Duration::from_secs(60 * 60 * if cfg!(debug_assertions) { 1 } else { 24 }),
                // if checked since `now`, then don't retain (or time travel, but whatever)
                Err(_) => false,
            })
            .await
    }
}

use headers::AcceptEncoding;
use util::cmap::EntryValue;

#[async_trait::async_trait]
impl FileCache for MainFileCache {
    type File = CachedFile;
    type Meta = Metadata;

    async fn open(&self, path: &Path, accepts: Option<AcceptEncoding>) -> io::Result<Self::File> {
        let mut last_modified = None;

        match self.map.get_cloned(path).await {
            Some(file) => {
                let dur = SystemTime::now().duration_since(file.last_checked);

                match dur {
                    Err(_) => {
                        log::warn!("Duration calculation failed, time reversed?");
                    }
                    // recheck every 15 seconds in debug, 2 minutes in release (TODO: Increase?)
                    Ok(dur) if dur > Duration::from_secs(if cfg!(debug_assertions) { 15 } else { 120 }) => {
                        last_modified = Some(file.last_modified);
                    }
                    Ok(_) => {
                        let encoding = match accepts.and_then(|a| {
                            // prefer best
                            let mut encodings = a.sorted_encodings();
                            let preferred = encodings.next();
                            encodings.find(|e| *e == file.best).or(preferred)
                        }) {
                            None | Some(ContentCoding::COMPRESS | ContentCoding::IDENTITY) => {
                                ContentCoding::IDENTITY
                            }
                            Some(encoding) => encoding,
                        };

                        let file = CachedFile {
                            pos: 0,
                            last_modified: file.last_modified,
                            encoding,
                            buf: match encoding {
                                ContentCoding::BROTLI => file.brotli.clone(),
                                ContentCoding::DEFLATE => file.deflate.clone(),
                                ContentCoding::GZIP => file.gzip.clone(),
                                ContentCoding::IDENTITY => file.iden.clone(),
                                ContentCoding::COMPRESS => unreachable!(),
                            },
                        };

                        log::trace!(
                            "Serving cached {:?} ({}) encoded file: {}",
                            encoding,
                            file.buf.len(),
                            path.display()
                        );

                        return Ok(file);
                    }
                }
            }
            None => {}
        }

        use tokio::io::AsyncReadExt;

        // WARNING: This will lock an entire shard while the file is processed,
        // avoiding duplicate processing.
        let EntryValue { lock, mut entry } = self.map.entry(path).await;

        let mut file = tokio::fs::File::open(path).await?;

        // get `now` time after opening as we last checked since opening it
        let now = SystemTime::now();

        let meta = file.metadata().await?;

        if !meta.is_file() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Not found"));
        }

        let mut do_read = true;

        if let Some(last_modified) = last_modified {
            if last_modified == meta.modified()? {
                use hashbrown::hash_map::RawEntryMut;

                if let RawEntryMut::Occupied(ref mut entry) = entry {
                    entry.get_mut().last_checked = SystemTime::now();
                    do_read = false;
                }
            }
        }

        if do_read {
            log::trace!("Loading in file to cache: {}", path.display());

            let len = meta.len();

            if len > (1024 * 1024 * 10) {
                log::warn!("Caching file larger than 10MB! {}", path.display());
            }

            let mut content = Vec::with_capacity(len as usize);
            file.read_to_end(&mut content).await?;

            let (brotli, deflate, gzip, best) = {
                use async_compression::{
                    tokio::bufread::{BrotliEncoder, DeflateEncoder, GzipEncoder},
                    Level,
                };

                let level = if cfg!(debug_assertions) {
                    Level::Fastest
                } else {
                    Level::Best
                };

                let mut brotli_buffer = Vec::new();
                let mut deflate_buffer = Vec::new();
                let mut gzip_buffer = Vec::new();

                let mut brotli = BrotliEncoder::with_quality(&content[..], level);
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

                let mut best = ContentCoding::BROTLI;
                let mut best_len = brotli_buffer.len();

                if deflate_buffer.len() < best_len {
                    best = ContentCoding::DEFLATE;
                    best_len = deflate_buffer.len();
                }

                if gzip_buffer.len() < best_len {
                    best = ContentCoding::GZIP;
                }

                log::trace!(
                    "Brotli: {}, Deflate: {}, Gzip: {}",
                    brotli_buffer.len(),
                    deflate_buffer.len(),
                    gzip_buffer.len()
                );

                (brotli_buffer, deflate_buffer, gzip_buffer, best)
            };

            log::trace!(
                "Inserting {} bytes into file cache from {}",
                (content.len() + brotli.len() + deflate.len() + gzip.len()),
                path.display(),
            );

            entry.insert(
                path.to_path_buf(),
                CacheEntry {
                    best,
                    // NOTE: Arc::from(vec) does not overallocate, so shrink_to_fit() is not needed
                    iden: Arc::from(content),
                    brotli: Arc::from(brotli),
                    deflate: Arc::from(deflate),
                    gzip: Arc::from(gzip),
                    last_modified: meta.modified()?,
                    last_checked: now,
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

    #[inline]
    async fn file_metadata(&self, file: &Self::File) -> io::Result<Self::Meta> {
        Ok(Metadata {
            len: file.buf.len() as u64,
            last_modified: file.last_modified,
            is_dir: false,
        })
    }
}
