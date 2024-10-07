use std::io::{self, SeekFrom};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime};

use futures::FutureExt;
use tokio::io::{AsyncRead, AsyncSeek, ReadBuf};

use ftl::fs::{EncodedFile, FileCache, FileMetadata};
use ftl::headers::accept_encoding::{AcceptEncoding, ContentEncoding};

use bytes::Bytes;

// TODO: use FilterEncoding anywhere?

use crate::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Metadata {
    is_dir: bool,
    len: u64,
    last_modified: SystemTime,
}

#[rustfmt::skip]
impl FileMetadata for Metadata {
    #[inline] fn is_dir(&self) -> bool { self.is_dir }
    #[inline] fn len(&self) -> u64 { self.len }
    #[inline] fn modified(&self) -> io::Result<SystemTime> { Ok(self.last_modified) }
    #[inline] fn blksize(&self) -> u64 {
        const SMALL_FILE_BLOCK_SIZE: u64 = 1024 * 8;
        const LARGE_FILE_BLOCK_SIZE: u64 = 1024 * 32;

        if self.len > SMALL_FILE_BLOCK_SIZE {
            LARGE_FILE_BLOCK_SIZE
        } else {
            SMALL_FILE_BLOCK_SIZE
        }
    }
}

#[derive(Clone)]
pub struct CachedFile {
    buf: Bytes,
    pos: u64,
    encoding: ContentEncoding,
    last_modified: SystemTime,
}

impl EncodedFile for CachedFile {
    fn encoding(&self) -> ContentEncoding {
        self.encoding
    }

    fn full(&self) -> Option<Bytes> {
        Some(self.buf.clone())
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
    preferred: [ContentEncoding; 5],

    identity: Bytes,
    brotli: Bytes,
    gzip: Bytes,
    deflate: Bytes,
    zstd: Bytes,

    last_modified: SystemTime,
    last_checked: SystemTime,
}

#[derive(Default)]
pub struct StaticFileCache {
    map: scc::HashMap<PathBuf, CacheEntry, sdk::FxRandomState2>,
}

use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use std::sync::LazyLock;

static VARIABLE_PATTERNS: LazyLock<AhoCorasick> = LazyLock::new(|| {
    AhoCorasickBuilder::new()
        .build([
            /*0*/ "__CONFIG__",
            /*1*/ "__BASE_URL__",
            /*2*/ "__SERVER_NAME__",
            /*3*/ "__CDN_DOMAIN__",
        ])
        .unwrap()
});

impl StaticFileCache {
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

    pub async fn process(&self, state: &GatewayServerState, path: &Path, mut file: Vec<u8>) -> Vec<u8> {
        // if HTML file *or* manifest.json
        if matches!((path.extension(), path.file_stem()), (Some(ext), Some(stem)) if ext == "html" || (ext == "json" && stem == "manifest"))
        {
            file = self.do_process(state, file);
        }

        file
    }

    pub fn do_process(&self, state: &GatewayServerState, mut file: Vec<u8>) -> Vec<u8> {
        let mut new_file = Vec::new();

        let c = state.config_full();

        let mut last_index = 0;
        for m in VARIABLE_PATTERNS.find_iter(&file) {
            new_file.extend_from_slice(&file[last_index..m.start()]);

            last_index = m.end();

            match m.pattern().as_u32() {
                0 => {
                    serde_json::to_writer(
                        &mut new_file,
                        &sdk::models::ServerConfig {
                            hcaptcha_sitekey: c.shared.hcaptcha_sitekey,
                            cdn: c.shared.cdn_domain.as_str().into(),
                            min_age: c.shared.minimum_age,
                            secure: c.shared.secure_web,
                            camo: c.shared.camo_enable,
                            limits: sdk::models::ServerLimits {
                                max_upload_size: c.shared.max_upload_size,
                                max_avatar_size: c.shared.max_avatar_size,
                                max_banner_size: c.shared.max_banner_size,
                                max_avatar_pixels: c.shared.max_avatar_pixels,
                                max_banner_pixels: c.shared.max_banner_pixels,
                                avatar_width: c.shared.avatar_width,
                                banner_width: c.shared.banner_width,
                                banner_height: c.shared.banner_height,
                            },
                        },
                    )
                    .unwrap();
                }

                1 => new_file.extend_from_slice(String::as_bytes(&format!(
                    "http{}://{}",
                    if c.shared.secure_web { "s" } else { "" },
                    c.shared.base_domain
                ))),
                2 => new_file.extend_from_slice(c.shared.server_name.as_bytes()),
                3 => new_file.extend_from_slice(c.shared.cdn_domain.as_bytes()),
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

impl FileCache<GatewayServerState> for StaticFileCache {
    type File = CachedFile;
    type Meta = Metadata;

    async fn clear(&self, _state: &GatewayServerState) {
        self.map.clear_async().await;
    }

    async fn metadata(&self, path: &Path, _state: &GatewayServerState) -> io::Result<Self::Meta> {
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
                let meta = tokio::fs::metadata(path).boxed().await?;

                Ok(Metadata {
                    is_dir: meta.is_dir(),
                    len: meta.len(),
                    last_modified: meta.modified()?,
                })
            }
        }
    }

    #[inline]
    async fn file_metadata(&self, file: &Self::File, _state: &GatewayServerState) -> io::Result<Self::Meta> {
        Ok(Metadata {
            len: file.buf.len() as u64,
            last_modified: file.last_modified,
            is_dir: false,
        })
    }

    async fn open(
        &self,
        path: &Path,
        accepts: Option<AcceptEncoding>,
        state: &GatewayServerState,
    ) -> io::Result<Self::File> {
        loop {
            let mut last_modified = None;

            if let Some(file) = self.map.read_async(path, |_, file| file.clone()).await {
                match file.last_checked.elapsed() {
                    Err(_) => log::warn!("Duration calculation failed, time reversed?"),

                    // file is okay but out of date, check if it's been modified
                    Ok(dur) if dur > state.config().shared.fs_cache_interval => {
                        last_modified = Some(file.last_modified);
                    }

                    Ok(_) => {
                        let mut encoding = ContentEncoding::Identity;

                        if let Some(a) = accepts {
                            for preferred in file.preferred {
                                if a.allows(preferred) {
                                    encoding = preferred;
                                    break;
                                }
                            }
                        }

                        let f = CachedFile {
                            pos: 0,
                            last_modified: file.last_modified,
                            encoding,
                            buf: match encoding {
                                ContentEncoding::Zstd => file.zstd,
                                ContentEncoding::Brotli => file.brotli,
                                ContentEncoding::Deflate => file.deflate,
                                ContentEncoding::Gzip => file.gzip,
                                ContentEncoding::Identity => file.identity,
                            },
                        };

                        log::trace!(
                            "Serving cached {encoding:?} ({}) (preferred {:?}) encoded file: {}",
                            f.buf.len(),
                            file.preferred,
                            path.display()
                        );

                        return Ok(f);
                    }
                }
            }

            let () = self.do_open(state, path, last_modified).boxed().await?;
        }
    }
}

impl StaticFileCache {
    pub async fn do_open(
        &self,
        state: &GatewayServerState,
        path: &Path,
        prev_last_modified: Option<SystemTime>,
    ) -> io::Result<()> {
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

        let last_modified = meta.modified()?;

        if matches!(prev_last_modified, Some(prev) if prev == last_modified) {
            use scc::hash_map::Entry;

            if let Entry::Occupied(ref mut entry) = entry {
                entry.get_mut().last_checked = now;
                return Ok(()); // try again, file is up to date
            }
        }

        log::trace!("Loading file into cache: {}", path.display());

        let len = meta.len();

        if len > (1024 * 1024 * 10) {
            log::warn!("Caching file larger than 10MB! {}", path.display());
        }

        let mut identity = Vec::with_capacity(len as usize);
        file.read_to_end(&mut identity).await?;

        // apply pre-processing
        identity = self.process(state, path, identity).await;

        use async_compression::{
            tokio::bufread::{BrotliEncoder, DeflateEncoder, GzipEncoder, ZstdEncoder},
            Level,
        };

        let level = if cfg!(debug_assertions) { Level::Fastest } else { Level::Best };

        let deflate_task = async {
            log::trace!("Compressing with Deflate");
            let mut deflate_buffer: Vec<u8> = Vec::with_capacity(128);
            let mut deflate = DeflateEncoder::with_quality(&identity[..], level);
            deflate.read_to_end(&mut deflate_buffer).await?;
            Ok::<_, std::io::Error>(deflate_buffer)
        };

        let gzip_task = async {
            log::trace!("Compressing with GZip");
            let mut gzip_buffer: Vec<u8> = Vec::with_capacity(128);
            let mut gzip = GzipEncoder::with_quality(&identity[..], level);
            gzip.read_to_end(&mut gzip_buffer).await?;
            Ok::<_, std::io::Error>(gzip_buffer)
        };

        let brotli_task = async {
            log::trace!("Compressing with Brotli");
            let mut brotli_buffer: Vec<u8> = Vec::with_capacity(128);
            let mut brotli = BrotliEncoder::with_quality(&identity[..], level);
            brotli.read_to_end(&mut brotli_buffer).await?;
            Ok::<_, std::io::Error>(brotli_buffer)
        };

        let zstd_task = async {
            // See https://issues.chromium.org/issues/41493659:
            //  "For memory usage reasons, Chromium limits the window size to 8MB"
            // See https://datatracker.ietf.org/doc/html/rfc8878#name-window-descriptor
            //  "For improved interoperability, it's recommended for decoders to support values
            //  of Window_Size up to 8 MB and for encoders not to generate frames requiring a
            //  Window_Size larger than 8 MB."
            // Level 17 in zstd (as of v1.5.6) is the first level with a window size of 8 MB (2^23):
            // https://github.com/facebook/zstd/blob/v1.5.6/lib/compress/clevels.h#L25-L51
            // Set the parameter for all levels >= 17. This will either have no effect (but reduce
            // the risk of future changes in zstd) or limit the window log to 8MB.
            let needs_window_limit = match level {
                Level::Best => true, // 20
                Level::Precise(level) => level >= 17,
                _ => false,
            };

            // The parameter is not set for levels below 17 as it will increase the window size
            // for those levels.
            let params: &[_] =
                if needs_window_limit { &[async_compression::zstd::CParameter::window_log(23)] } else { &[] };

            log::trace!("Compressing with Zstd");
            let mut zstd_buffer: Vec<u8> = Vec::with_capacity(128);
            let mut zstd = ZstdEncoder::with_quality_and_params(&identity[..], level, params);
            zstd.read_to_end(&mut zstd_buffer).await?;
            Ok::<_, std::io::Error>(zstd_buffer)
        };

        let (zstd, brotli, deflate, gzip) = tokio::try_join!(zstd_task, brotli_task, deflate_task, gzip_task)?;

        let mut preferred = [
            (ContentEncoding::Zstd, zstd.len()),
            (ContentEncoding::Brotli, brotli.len()),
            (ContentEncoding::Deflate, deflate.len()),
            (ContentEncoding::Gzip, gzip.len()),
            (ContentEncoding::Identity, identity.len()),
        ];

        // sorts by length, smallest first
        preferred.sort_by_key(|(_, len)| *len);

        log::trace!(
            "Zstd: {}, Brotli: {}, Deflate: {}, Gzip: {}",
            zstd.len(),
            brotli.len(),
            deflate.len(),
            gzip.len()
        );

        log::trace!(
            "Inserting {} bytes into file cache from {}",
            (identity.len() + zstd.len() + brotli.len() + deflate.len() + gzip.len()),
            path.display(),
        );

        // NOTE: consumes entry
        entry.insert_entry(CacheEntry {
            last_modified,
            last_checked: now,

            identity: identity.into(),
            brotli: brotli.into(),
            deflate: deflate.into(),
            gzip: gzip.into(),
            zstd: zstd.into(),
            preferred: preferred.map(|(encoding, _)| encoding),
        });

        Ok::<(), io::Error>(())
    }
}
