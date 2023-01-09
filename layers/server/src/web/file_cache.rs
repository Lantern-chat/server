use std::borrow::Cow;
use std::io::{self, SeekFrom};
use std::path::{Path, PathBuf};
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime};
use std::{pin::Pin, sync::Arc};

use tokio::io::{AsyncRead, AsyncSeek, ReadBuf};
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
    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
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

// #[cfg(debug_assertions)]
// impl Drop for CacheEntry {
//     fn drop(&mut self) {
//         let count = Arc::strong_count(&self.iden)
//             + Arc::strong_count(&self.brotli)
//             + Arc::strong_count(&self.gzip)
//             + Arc::strong_count(&self.deflate);

//         log::debug!("Dropping cached file! References: {count}");
//     }
// }

#[derive(Default)]
pub struct MainFileCache {
    map: CHashMap<PathBuf, CacheEntry>,
}

impl MainFileCache {
    pub async fn cleanup(&self) {
        let now = SystemTime::now();

        self.map
            .retain(|_, file| match now.duration_since(file.last_checked) {
                // retain if duration since last checked is less than 10 minutes (debug) or 24 hours (release)
                Ok(dur) => dur < Duration::from_secs(60 * if cfg!(debug_assertions) { 10 } else { 24 * 60 }),
                // if checked since `now`, then don't retain (or time travel, but whatever)
                Err(_) => false,
            })
            .await
    }
}

use headers::AcceptEncoding;
use util::cmap::EntryValue;

use crate::ServerState;

#[async_trait::async_trait]
impl FileCache<ServerState> for MainFileCache {
    type File = CachedFile;
    type Meta = Metadata;

    async fn clear(&self, _state: &ServerState) {
        self.map.retain(|_, _| false).await
    }

    async fn open(
        &self,
        path: &Path,
        accepts: Option<AcceptEncoding>,
        state: &ServerState,
    ) -> io::Result<Self::File> {
        let mut last_modified = None;

        if let Some(file) = self.map.get_cloned(path).await {
            let dur = SystemTime::now().duration_since(file.last_checked);

            match dur {
                Err(_) => log::warn!("Duration calculation failed, time reversed?"),
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
                        None | Some(ContentCoding::COMPRESS | ContentCoding::IDENTITY) => ContentCoding::IDENTITY,
                        Some(encoding) => encoding,
                    };

                    let file = CachedFile {
                        pos: 0,
                        last_modified: file.last_modified,
                        encoding,
                        buf: match encoding {
                            ContentCoding::BROTLI => file.brotli,
                            ContentCoding::DEFLATE => file.deflate,
                            ContentCoding::GZIP => file.gzip,
                            ContentCoding::IDENTITY => file.iden,
                            ContentCoding::COMPRESS => unreachable!(),
                        },
                    };

                    log::trace!(
                        "Serving cached {encoding:?} ({}) encoded file: {}",
                        file.buf.len(),
                        path.display()
                    );

                    return Ok(file);
                }
            }
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

            content = self.process(state, path, content).await;

            let (brotli, deflate, gzip, best) = {
                use async_compression::{
                    tokio::bufread::{BrotliEncoder, DeflateEncoder, GzipEncoder},
                    Level,
                };

                let level = if cfg!(debug_assertions) { Level::Fastest } else { Level::Best };

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

        self.open(path, accepts, state).await
    }

    async fn metadata(&self, path: &Path, _state: &ServerState) -> io::Result<Self::Meta> {
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
    async fn file_metadata(&self, file: &Self::File, _state: &ServerState) -> io::Result<Self::Meta> {
        Ok(Metadata {
            len: file.buf.len() as u64,
            last_modified: file.last_modified,
            is_dir: false,
        })
    }
}

use aho_corasick::{AhoCorasick, AhoCorasickBuilder};

lazy_static::lazy_static! {
    static ref VARIABLE_PATTERNS: AhoCorasick = AhoCorasickBuilder::new().dfa(true).build([
        /*0*/ "__CONFIG__",
        /*1*/ "__BASE_URL__",
        /*2*/ "__SERVER_NAME__",
    ]);
}

impl MainFileCache {
    pub async fn process(&self, state: &ServerState, path: &Path, mut file: Vec<u8>) -> Vec<u8> {
        let do_process = match (path.extension(), path.file_stem()) {
            // if HTML file *or* manifest.json
            (Some(ext), Some(stem)) => ext == "html" || (ext == "json" && stem == "manifest"),
            _ => false,
        };

        if do_process {
            file = self.do_process(state, file);
        }

        file
    }

    pub fn do_process(&self, state: &ServerState, mut file: Vec<u8>) -> Vec<u8> {
        let mut new_file = Vec::new();

        let c = state.config();

        let mut last_index = 0;
        for m in VARIABLE_PATTERNS.find_iter(&file) {
            new_file.extend_from_slice(&file[last_index..m.start()]);

            last_index = m.end();

            match m.pattern() {
                0 => {
                    serde_json::to_writer(
                        &mut new_file,
                        &sdk::models::ServerConfig {
                            hcaptcha_sitekey: c.services.hcaptcha_sitekey.clone(),
                            cdn: c.web.cdn_domain.clone(),
                            min_age: c.account.min_age,
                            secure: c.web.secure,
                            camo: c.web.camo,
                            limits: sdk::models::ServerLimits {
                                max_upload_size: c.upload.max_upload_size,
                                max_avatar_size: c.upload.max_avatar_size as u32,
                                max_banner_size: c.upload.max_banner_size as u32,
                                max_avatar_pixels: c.upload.max_avatar_pixels,
                                max_banner_pixels: c.upload.max_banner_pixels,
                                avatar_width: c.upload.avatar_width,
                                banner_width: c.upload.banner_width,
                                banner_height: c.upload.banner_height,
                            },
                        },
                    )
                    .unwrap();
                }
                1 => new_file.extend_from_slice(c.web.base_url().as_bytes()),
                2 => new_file.extend_from_slice(c.general.server_name.as_bytes()),
                _ => log::error!("Unreachable replacement"),
            }
        }

        if last_index > 0 {
            new_file.extend_from_slice(&file[last_index..]);

            file = new_file;
        }

        file
    }
}
