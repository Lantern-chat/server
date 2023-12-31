use std::io::{self, SeekFrom};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime};
use triomphe::Arc;

use tokio::io::{AsyncRead, AsyncSeek, ReadBuf};

use ftl::fs::{EncodedFile, FileCache, FileMetadata};

use headers::ContentCoding;

#[derive(Clone, Copy, Debug)]
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
    fn poll_read(mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let pos = self.pos as usize;
        let end = (pos + buf.remaining()).min(self.buf.len());

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

    #[inline]
    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        Poll::Ready(Ok(self.pos))
    }
}

#[derive(Clone)]
pub struct CacheEntry {
    best: ContentCoding,
    identity: Arc<[u8]>,
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
    map: scc::HashMap<PathBuf, CacheEntry, ahash::RandomState>,
}

impl MainFileCache {
    pub async fn cleanup(&self, max_age: Duration) {
        let now = SystemTime::now();

        self.map
            .retain_async(|_, file| match now.duration_since(file.last_checked) {
                Ok(dur) => dur < max_age,
                // if the file has been checked since now, then it's likely valid
                Err(_) => true,
            })
            .await;
    }
}

use headers::AcceptEncoding;

use crate::prelude::*;

impl FileCache<ServerState> for MainFileCache {
    type File = CachedFile;
    type Meta = Metadata;

    async fn clear(&self, _state: &ServerState) {
        self.map.retain_async(|_, _| false).await;
    }

    async fn open(
        &self,
        path: &Path,
        accepts: Option<AcceptEncoding>,
        state: &ServerState,
    ) -> io::Result<Self::File> {
        loop {
            let mut last_modified = None;

            if let Some(file) = self.map.read_async(path, |_, file| file.clone()).await {
                match file.last_checked.elapsed() {
                    Err(_) => log::warn!("Duration calculation failed, time reversed?"),
                    Ok(dur) if dur > state.config().shared.fs_cache_interval => {
                        last_modified = Some(file.last_modified);
                    }
                    Ok(_) => {
                        let encoding = match accepts.and_then(|a| {
                            // prefer best
                            #[cfg(feature = "brotli")]
                            let mut encodings = a.sorted_encodings();

                            // TODO: make this filtering on feature cleaner
                            #[cfg(not(feature = "brotli"))]
                            let mut encodings = a.sorted_encodings().filter(|c| *c != ContentCoding::BROTLI);

                            let preferred = encodings.next();
                            encodings.find(|e| *e == file.best).or(preferred)
                        }) {
                            None | Some(ContentCoding::COMPRESS | ContentCoding::IDENTITY) => {
                                ContentCoding::IDENTITY
                            }
                            Some(encoding) => encoding,
                        };

                        let f = CachedFile {
                            pos: 0,
                            last_modified: file.last_modified,
                            encoding,
                            buf: match encoding {
                                ContentCoding::BROTLI => file.brotli,
                                ContentCoding::DEFLATE => file.deflate,
                                ContentCoding::GZIP => file.gzip,
                                ContentCoding::IDENTITY => file.identity,
                                ContentCoding::COMPRESS => unreachable!(),
                            },
                        };

                        log::trace!(
                            "Serving cached {encoding:?} ({}) (best {:?}) encoded file: {}",
                            f.buf.len(),
                            file.best,
                            path.display()
                        );

                        return Ok(f);
                    }
                }
            }

            use tokio::io::AsyncReadExt;

            // lock entry
            let mut entry = self.map.entry_async(path.to_owned()).await;

            let mut file = tokio::fs::File::open(path).await?;

            // get `now` time after opening as we last checked since opening it
            let now = SystemTime::now();

            let meta = file.metadata().await?;

            if !meta.is_file() {
                return Err(io::Error::new(io::ErrorKind::NotFound, "Not found"));
            }

            if let Some(last_modified) = last_modified {
                if last_modified == meta.modified()? {
                    use scc::hash_map::Entry;

                    if let Entry::Occupied(ref mut entry) = entry {
                        entry.get_mut().last_checked = now;
                        continue; // try again, file is up to date
                    }
                }
            }

            log::trace!("Loading file into cache: {}", path.display());

            let len = meta.len();

            if len > (1024 * 1024 * 10) {
                log::warn!("Caching file larger than 10MB! {}", path.display());
            }

            let mut content = Vec::with_capacity(len as usize);
            file.read_to_end(&mut content).await?;

            content = self.process(state, path, content).await;

            struct CompressionResults {
                brotli: Vec<u8>,
                deflate: Vec<u8>,
                gzip: Vec<u8>,
                best: ContentCoding,
            }

            let compressed = {
                use async_compression::{
                    tokio::bufread::{DeflateEncoder, GzipEncoder},
                    Level,
                };

                let level = if cfg!(debug_assertions) { Level::Fastest } else { Level::Best };

                let deflate_task = async {
                    log::trace!("Compressing with Deflate");
                    let mut deflate_buffer: Vec<u8> = Vec::with_capacity(128);
                    let mut deflate = DeflateEncoder::with_quality(&content[..], level);
                    deflate.read_to_end(&mut deflate_buffer).await?;
                    Ok::<_, std::io::Error>(deflate_buffer)
                };

                let gzip_task = async {
                    log::trace!("Compressing with GZip");
                    let mut gzip_buffer: Vec<u8> = Vec::with_capacity(128);
                    let mut gzip = GzipEncoder::with_quality(&content[..], level);
                    gzip.read_to_end(&mut gzip_buffer).await?;
                    Ok::<_, std::io::Error>(gzip_buffer)
                };

                #[cfg(feature = "brotli")]
                let brotli_task = async {
                    use async_compression::tokio::bufread::BrotliEncoder;

                    log::trace!("Compressing with Brotli");
                    let mut brotli_buffer: Vec<u8> = Vec::with_capacity(128);
                    let mut brotli = BrotliEncoder::with_quality(&content[..], level);
                    brotli.read_to_end(&mut brotli_buffer).await?;
                    Ok::<_, std::io::Error>(brotli_buffer)
                };

                #[cfg(not(feature = "brotli"))]
                let brotli_task = async { Ok::<_, std::io::Error>(Vec::new()) };

                let (brotli, deflate, gzip) = tokio::try_join!(brotli_task, deflate_task, gzip_task)?;

                let mut best = ContentCoding::IDENTITY;
                let mut best_len = content.len();

                #[cfg(feature = "brotli")]
                if brotli.len() < best_len {
                    best = ContentCoding::BROTLI;
                    best_len = brotli.len();
                }

                if deflate.len() < best_len {
                    best = ContentCoding::DEFLATE;
                    best_len = deflate.len();
                }

                if gzip.len() < best_len {
                    best = ContentCoding::GZIP;
                }

                log::trace!(
                    "Brotli: {}, Deflate: {}, Gzip: {}",
                    brotli.len(),
                    deflate.len(),
                    gzip.len()
                );

                CompressionResults {
                    brotli,
                    deflate,
                    gzip,
                    best,
                }
            };

            log::trace!(
                "Inserting {} bytes into file cache from {}",
                (content.len() + compressed.brotli.len() + compressed.deflate.len() + compressed.gzip.len()),
                path.display(),
            );

            // NOTE: consumes entry
            entry.insert_entry(CacheEntry {
                last_modified: meta.modified()?,
                last_checked: now,

                // NOTE: Arc::from(vec) does not overallocate, so shrink_to_fit() is not needed
                identity: Arc::from(content),
                brotli: Arc::from(compressed.brotli),
                deflate: Arc::from(compressed.deflate),
                gzip: Arc::from(compressed.gzip),
                best: compressed.best,
            });
        }
    }

    async fn metadata(&self, path: &Path, _state: &ServerState) -> io::Result<Self::Meta> {
        match self
            .map
            .read_async(path, |_, file| Metadata {
                len: file.identity.len() as u64,
                last_modified: file.last_modified,
                is_dir: false,
            })
            .await
        {
            Some(file) => Ok(file),
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
    static ref VARIABLE_PATTERNS: AhoCorasick = AhoCorasickBuilder::new().build([
        /*0*/ "__CONFIG__",
        /*1*/ "__BASE_URL__",
        /*2*/ "__SERVER_NAME__",
        /*3*/ "__CDN_DOMAIN__",
    ]).unwrap();
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

        //let c = state.config();

        let mut last_index = 0;
        for m in VARIABLE_PATTERNS.find_iter(&file) {
            new_file.extend_from_slice(&file[last_index..m.start()]);

            last_index = m.end();

            // match m.pattern().as_u32() {
            //     0 => {
            //         serde_json::to_writer(
            //             &mut new_file,
            //             &sdk::models::ServerConfig {
            //                 hcaptcha_sitekey: c.services.hcaptcha_sitekey.clone(),
            //                 cdn: c.web.cdn_domain.clone(),
            //                 min_age: c.account.min_age,
            //                 secure: c.web.secure,
            //                 camo: c.web.camo,
            //                 limits: sdk::models::ServerLimits {
            //                     max_upload_size: c.upload.max_upload_size,
            //                     max_avatar_size: c.upload.max_avatar_size as u32,
            //                     max_banner_size: c.upload.max_banner_size as u32,
            //                     max_avatar_pixels: c.upload.max_avatar_pixels,
            //                     max_banner_pixels: c.upload.max_banner_pixels,
            //                     avatar_width: c.upload.avatar_width,
            //                     banner_width: c.upload.banner_width,
            //                     banner_height: c.upload.banner_height,
            //                 },
            //             },
            //         )
            //         .unwrap();
            //     }
            //     1 => new_file.extend_from_slice(c.web.base_url().as_bytes()),
            //     2 => new_file.extend_from_slice(c.general.server_name.as_bytes()),
            //     3 => new_file.extend_from_slice(c.web.cdn_domain.as_bytes()),
            //     _ => log::error!("Unreachable replacement"),
            // }
        }

        if last_index > 0 {
            new_file.extend_from_slice(&file[last_index..]);

            file = new_file;
        }

        file
    }
}
