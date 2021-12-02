use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::{fs::Metadata, io, time::Instant};

use bytes::{Bytes, BytesMut};
use tokio::fs::File as TkFile;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};

use headers::{
    AcceptEncoding, AcceptRanges, ContentCoding, ContentEncoding, ContentLength, ContentRange, ContentType,
    HeaderMap, HeaderMapExt, HeaderValue, IfModifiedSince, IfRange, IfUnmodifiedSince, LastModified, Range,
    TransferEncoding,
};
use http::{header::TRAILER, Method, Response, StatusCode};
use hyper::Body;
use percent_encoding::percent_decode_str;

use super::{Reply, Route};

use async_trait::async_trait;

// TODO: https://github.com/magiclen/entity-tag/blob/master/src/lib.rs
// https://github.com/pillarjs/send/blob/master/index.js
// https://github.com/jshttp/etag/blob/master/index.js

pub trait GenericFile: Unpin + AsyncRead + AsyncSeek + Send + 'static {}
impl<T> GenericFile for T where T: Unpin + AsyncRead + AsyncSeek + Send + 'static {}

pub trait EncodedFile {
    fn encoding(&self) -> ContentCoding;
}

impl EncodedFile for TkFile {
    #[inline]
    fn encoding(&self) -> ContentCoding {
        ContentCoding::IDENTITY
    }
}

pub trait FileMetadata {
    fn is_dir(&self) -> bool;
    fn len(&self) -> u64;
    fn modified(&self) -> io::Result<SystemTime>;
    fn blksize(&self) -> u64;
}

impl FileMetadata for Metadata {
    #[inline]
    fn is_dir(&self) -> bool {
        Metadata::is_dir(self)
    }

    #[inline]
    fn modified(&self) -> io::Result<SystemTime> {
        Metadata::modified(self)
    }

    #[inline]
    fn len(&self) -> u64 {
        Metadata::len(self)
    }

    #[cfg(unix)]
    #[inline]
    fn blksize(&self) -> u64 {
        use std::os::unix::fs::MetadataExt;

        MetadataExt::blksize(self)
    }

    #[cfg(not(unix))]
    #[inline]
    fn blksize(&self) -> u64 {
        0
    }
}

#[async_trait]
pub trait FileCache {
    type File: GenericFile + EncodedFile;
    type Meta: FileMetadata;

    async fn open(&self, path: &Path, accepts: Option<AcceptEncoding>) -> io::Result<Self::File>;
    async fn metadata(&self, path: &Path) -> io::Result<Self::Meta>;
    async fn file_metadata(&self, file: &Self::File) -> io::Result<Self::Meta>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoCache;

#[async_trait]
impl FileCache for NoCache {
    type File = TkFile;
    type Meta = Metadata;

    #[inline]
    async fn open(&self, path: &Path, _accepts: Option<AcceptEncoding>) -> io::Result<Self::File> {
        TkFile::open(path).await
    }

    #[inline]
    async fn metadata(&self, path: &Path) -> io::Result<Self::Meta> {
        tokio::fs::metadata(path).await
    }

    #[inline]
    async fn file_metadata(&self, file: &Self::File) -> io::Result<Self::Meta> {
        file.metadata().await
    }
}

#[derive(Debug)]
pub struct Conditionals {
    if_modified_since: Option<IfModifiedSince>,
    if_unmodified_since: Option<IfUnmodifiedSince>,
    if_range: Option<IfRange>,
    range: Option<Range>,
}

pub enum Cond {
    NoBody(Response<Body>),
    WithBody(Option<Range>),
}

impl Conditionals {
    pub fn new<S>(route: &Route<S>, range: Option<Range>) -> Conditionals {
        let req_headers = route.req.headers();
        Conditionals {
            range,
            if_modified_since: req_headers.typed_get(),
            if_unmodified_since: req_headers.typed_get(),
            if_range: req_headers.typed_get(),
        }
    }

    pub fn check(self, last_modified: Option<LastModified>) -> Cond {
        if let Some(since) = self.if_unmodified_since {
            let precondition = last_modified
                .map(|time| since.precondition_passes(time.into()))
                .unwrap_or(false);

            log::trace!(
                "if-unmodified-since? {:?} vs {:?} = {}",
                since,
                last_modified,
                precondition
            );

            if !precondition {
                return Cond::NoBody(StatusCode::PRECONDITION_FAILED.into_response());
            }
        }

        if let Some(since) = self.if_modified_since {
            log::trace!(
                "if-modified-since? header = {:?}, file = {:?}",
                since,
                last_modified
            );

            let unmodified = last_modified
                .map(|time| !since.is_modified(time.into()))
                // no last_modified means its always modified
                .unwrap_or(false);

            if unmodified {
                return Cond::NoBody(StatusCode::NOT_MODIFIED.into_response());
            }
        }

        if let Some(if_range) = self.if_range {
            log::trace!("if-range? {:?} vs {:?}", if_range, last_modified);
            let can_range = !if_range.is_modified(None, last_modified.as_ref());

            if !can_range {
                return Cond::WithBody(None);
            }
        }

        Cond::WithBody(self.range)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SanitizeError {
    #[error("Invalid Path")]
    InvalidPath,

    #[error("UTF-8 Error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}

pub fn sanitize_path(base: impl AsRef<Path>, tail: &str) -> Result<PathBuf, SanitizeError> {
    let base = base.as_ref();
    let mut buf = base.to_path_buf();
    let p = percent_decode_str(tail).decode_utf8()?;

    for seg in p.split('/') {
        if seg.starts_with("..") {
            log::warn!("dir: rejecting segment starting with '..'");
            return Err(SanitizeError::InvalidPath);
        }

        if seg.contains('\\') {
            log::warn!("dir: rejecting segment containing with backslash (\\)");
            return Err(SanitizeError::InvalidPath);
        }

        buf.push(seg);
    }

    //if !buf.starts_with(base) {
    //    log::warn!("dir: rejecting path that is not a child of base");
    //    return Err(SanitizeError::InvalidPath);
    //}

    Ok(buf)
}

const DEFAULT_READ_BUF_SIZE: u64 = 1024 * 32;

pub async fn file<S>(route: &Route<S>, path: impl AsRef<Path>, cache: &impl FileCache) -> impl Reply {
    file_reply(route, path, cache).await
}

pub async fn dir<S>(route: &Route<S>, base: impl AsRef<Path>, cache: &impl FileCache) -> impl Reply {
    let mut buf = match sanitize_path(base, route.tail()) {
        Ok(buf) => buf,
        Err(e) => return e.to_string().with_status(StatusCode::BAD_REQUEST).into_response(),
    };

    let is_dir = cache.metadata(&buf).await.map(|m| m.is_dir()).unwrap_or(false);

    if is_dir {
        log::debug!("dir: appending index.html to directory path");
        buf.push("index.html");
    }

    file_reply(route, buf, cache).await.into_response()
}

async fn file_reply<S>(route: &Route<S>, path: impl AsRef<Path>, cache: &impl FileCache) -> impl Reply {
    let path = path.as_ref();

    let range = route.header::<headers::Range>();

    // if a range is given, do not use pre-compression
    let accepts = match range {
        None => route.header::<AcceptEncoding>(),
        Some(_) => None,
    };

    let file = match cache.open(path, accepts).await {
        Ok(f) => f,
        Err(e) => {
            return match e.kind() {
                std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND.into_response(),
                std::io::ErrorKind::PermissionDenied => StatusCode::FORBIDDEN.into_response(),
                _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    };

    let metadata = match cache.file_metadata(&file).await {
        Ok(m) => m,
        Err(e) => {
            log::error!("Error retreiving file metadata: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // parse after opening the file handle to save time on open error
    let conditionals = Conditionals::new(route, range);

    let modified = match metadata.modified() {
        Err(_) => None,
        Ok(t) => Some(LastModified::from(t)),
    };

    let mut len = metadata.len();

    match conditionals.check(modified) {
        Cond::NoBody(resp) => resp,
        Cond::WithBody(range) => match bytes_range(range, len) {
            Err(_) => StatusCode::RANGE_NOT_SATISFIABLE
                .with_header(ContentRange::unsatisfied_bytes(len))
                .into_response(),

            Ok((start, end)) => {
                let sub_len = end - start;
                let buf_size = metadata.blksize().max(DEFAULT_READ_BUF_SIZE).min(len) as usize;
                let encoding = file.encoding();

                let mut resp = if route.method() == Method::GET {
                    Response::new(file_body(route.start, file, buf_size, (start, end)))
                } else {
                    Response::default()
                };

                if sub_len != len {
                    assert_eq!(encoding, ContentCoding::IDENTITY);

                    *resp.status_mut() = StatusCode::PARTIAL_CONTENT;
                    resp.headers_mut()
                        .typed_insert(ContentRange::bytes(start..end, len).expect("valid ContentRange"));

                    len = sub_len;
                }

                let mime = mime_guess::from_path(path).first_or_octet_stream();

                let headers = resp.headers_mut();

                if let Some(last_modified) = modified {
                    headers.typed_insert(last_modified);
                }

                if encoding != ContentCoding::IDENTITY {
                    headers.append(
                        http::header::CONTENT_ENCODING,
                        HeaderValue::from_static(encoding.to_static()),
                    );
                }

                headers.insert(TRAILER, HeaderValue::from_static("Server-Timing"));

                headers.typed_insert(ContentLength(len));
                headers.typed_insert(ContentType::from(mime));
                headers.typed_insert(AcceptRanges::bytes());

                resp
            }
        },
    }
}

pub struct BadRange;
pub fn bytes_range(range: Option<Range>, max_len: u64) -> Result<(u64, u64), BadRange> {
    use std::ops::Bound;

    match range.and_then(|r| r.iter().next()) {
        Some((start, end)) => {
            let start = match start {
                Bound::Unbounded => 0,
                Bound::Included(s) => s,
                Bound::Excluded(s) => s + 1,
            };

            let end = match end {
                Bound::Unbounded => max_len,
                Bound::Included(s) => s + (s != max_len) as u64,
                Bound::Excluded(s) => s,
            };

            if start < end && end <= max_len {
                Ok((start, end))
            } else {
                log::trace!("unsatisfiable byte range: {}-{}/{}", start, end, max_len);
                Err(BadRange)
            }
        }
        None => Ok((0, max_len)),
    }
}

use futures::Stream;
use std::io::SeekFrom;

use std::pin::Pin;

fn file_body(
    route_start: Instant,
    mut file: impl GenericFile,
    buf_size: usize,
    (start, end): (u64, u64),
) -> Body {
    //return Body::wrap_stream(file_stream(file, buf_size, (start, end)));

    let (mut sender, body) = Body::channel();

    tokio::spawn(async move {
        if start != 0 {
            if let Err(e) = file.seek(SeekFrom::Start(start)).await {
                log::error!("Error seeking file: {}", e);
                return sender.abort();
            }
        }

        let mut buf = BytesMut::new();
        let mut len = end - start;

        while len != 0 {
            // reserve at least buf_size
            if buf.capacity() - buf.len() < buf_size {
                buf.reserve(buf_size);
            }

            let n = match file.read_buf(&mut buf).await {
                Ok(n) => n as u64,
                Err(err) => {
                    log::debug!("file read error: {}", err);
                    return sender.abort();
                }
            };

            if n == 0 {
                log::warn!("file read found EOF before expected length: {}", len);
                break;
            }

            let mut chunk = buf.split().freeze();
            if n > len {
                chunk = chunk.split_to(len as usize);
                len = 0;
            } else {
                len -= n;
            }

            if let Err(e) = sender.send_data(chunk).await {
                log::trace!("Error sending file chunk: {}", e);
                return sender.abort();
            }
        }

        let elapsed = route_start.elapsed().as_secs_f64() * 1000.0;

        log::debug!("File transfer finished in {:.4}ms", elapsed);

        let mut trailers = HeaderMap::new();
        if let Ok(value) = HeaderValue::from_str(&format!("end;dur={:.4}", elapsed)) {
            trailers.insert("Server-Timing", value);

            if let Err(e) = sender.send_trailers(trailers).await {
                log::trace!("Error sending trailers: {}", e);
            }
        } else {
            log::trace!("Unable to create trailer value");
        }

        drop(sender);
    });

    body
}

/*
// TODO: Rewrite this with manual stream state machine for highest possible efficiency
// Take note of https://github.com/tokio-rs/tokio/blob/43bd11bf2fa4eaee84383ddbe4c750868f1bb684/tokio/src/io/seek.rs
fn file_stream(
    mut file: TkFile,
    buf_size: usize,
    (start, end): (u64, u64),
) -> impl Stream<Item = Result<Bytes, std::io::Error>> + Send {
    async_stream::stream! {
        if start != 0 {
            if let Err(e) = file.seek(SeekFrom::Start(start)).await {
                yield Err(e);
                return;
            }
        }

        let mut buf = BytesMut::new();
        let mut len = end - start;

        while len != 0 {
            // reserve at least buf_size
            if buf.capacity() - buf.len() < buf_size {
                buf.reserve(buf_size);
            }

            let n = match file.read_buf(&mut buf).await {
                Ok(n) => n as u64,
                Err(err) => {
                    log::debug!("file read error: {}", err);
                    yield Err(err);
                    break;
                }
            };

            if n == 0 {
                log::debug!("file read found EOF before expected length");
                break;
            }

            let mut chunk = buf.split().freeze();
            if n > len {
                chunk = chunk.split_to(len as usize);
                len = 0;
            } else {
                len -= n;
            }

            yield Ok(chunk);
        }
    }
}
*/

#[cfg(test)]
mod tests {
    use super::sanitize_path;
    use bytes::BytesMut;

    #[test]
    fn test_sanitize_path() {
        let base = "/var/www";

        fn p(s: &str) -> &::std::path::Path {
            s.as_ref()
        }

        assert_eq!(sanitize_path(base, "/foo.html").unwrap(), p("/var/www/foo.html"));

        // bad paths
        sanitize_path(base, "/../foo.html").expect_err("dot dot");

        sanitize_path(base, "/C:\\/foo.html").expect_err("C:\\");
    }
}
