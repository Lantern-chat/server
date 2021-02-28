use std::fs::Metadata;
use std::path::{Path, PathBuf};

use bytes::{Bytes, BytesMut};
use tokio::fs::File as TkFile;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use headers::{
    AcceptRanges, ContentLength, ContentRange, ContentType, HeaderMapExt, IfModifiedSince, IfRange,
    IfUnmodifiedSince, LastModified, Range,
};
use http::{Method, Response, StatusCode};
use hyper::Body;
use percent_encoding::percent_decode_str;

use super::{Reply, Route};

#[derive(Debug)]
pub struct Conditionals {
    if_modified_since: Option<IfModifiedSince>,
    if_unmodified_since: Option<IfUnmodifiedSince>,
    if_range: Option<IfRange>,
    range: Option<Range>,
}

enum Cond {
    NoBody(Response<Body>),
    WithBody(Option<Range>),
}

impl Conditionals {
    pub fn parse(route: &Route) -> Conditionals {
        let headers = route.req.headers();

        Conditionals {
            if_modified_since: headers.typed_get(),
            if_unmodified_since: headers.typed_get(),
            if_range: headers.typed_get(),
            range: headers.typed_get(),
        }
    }

    fn check(self, last_modified: Option<LastModified>) -> Cond {
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

const DEFAULT_READ_BUF_SIZE: usize = 8_192;

fn optimal_buf_size(metadata: &Metadata) -> usize {
    let block_size = get_block_size(metadata);

    // If file length is smaller than block size, don't waste space
    // reserving a bigger-than-needed buffer.
    std::cmp::min(block_size as u64, metadata.len()) as usize
}

#[cfg(unix)]
fn get_block_size(metadata: &Metadata) -> usize {
    use std::os::unix::fs::MetadataExt;
    //TODO: blksize() returns u64, should handle bad cast...
    //(really, a block size bigger than 4gb?)

    // Use device blocksize unless it's really small.
    std::cmp::max(metadata.blksize() as usize, DEFAULT_READ_BUF_SIZE)
}

#[cfg(not(unix))]
fn get_block_size(_metadata: &Metadata) -> usize {
    DEFAULT_READ_BUF_SIZE
}

pub async fn file(route: &Route, path: impl AsRef<Path>) -> impl Reply {
    file_reply(route, path).await
}

pub async fn dir(route: &Route, base: impl AsRef<Path>) -> impl Reply {
    let mut buf = match sanitize_path(base, route.tail()) {
        Ok(buf) => buf,
        Err(e) => {
            return e
                .to_string()
                .with_status(StatusCode::BAD_REQUEST)
                .into_response()
        }
    };

    let is_dir = tokio::fs::metadata(&buf)
        .await
        .map(|m| m.is_dir())
        .unwrap_or(false);

    if is_dir {
        log::debug!("dir: appending index.html to directory path");
        buf.push("index.html");
    }

    file_reply(route, buf).await.into_response()
}

async fn file_reply(route: &Route, path: impl AsRef<Path>) -> impl Reply {
    let path = path.as_ref();

    let file = match TkFile::open(path).await {
        Ok(f) => f,
        Err(e) => {
            return match e.kind() {
                std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND.into_response(),
                std::io::ErrorKind::PermissionDenied => StatusCode::FORBIDDEN.into_response(),
                _ => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
    };

    let metadata = match file.metadata().await {
        Ok(m) => m,
        Err(e) => {
            log::error!("Error retreiving file metadata: {}", e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // parse after opening the file handle to save time on open error
    let conditionals = Conditionals::parse(route);

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
                let buf_size = optimal_buf_size(&metadata);

                let mut resp = if route.method() == &Method::GET {
                    Response::new(Body::wrap_stream(file_stream(file, buf_size, (start, end))))
                } else {
                    Response::default()
                };

                if sub_len != len {
                    *resp.status_mut() = StatusCode::PARTIAL_CONTENT;
                    resp.headers_mut().typed_insert(
                        ContentRange::bytes(start..end, len).expect("valid ContentRange"),
                    );

                    len = sub_len;
                }

                let mime = mime_guess::from_path(path).first_or_octet_stream();

                if let Some(last_modified) = modified {
                    resp.headers_mut().typed_insert(last_modified);
                }

                resp.with_header(ContentLength(len))
                    .with_header(ContentType::from(mime))
                    .with_header(AcceptRanges::bytes())
                    .into_response()
            }
        },
    }
}

struct BadRange;
fn bytes_range(range: Option<Range>, max_len: u64) -> Result<(u64, u64), BadRange> {
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
                Bound::Included(s) => s + 1,
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

        assert_eq!(
            sanitize_path(base, "/foo.html").unwrap(),
            p("/var/www/foo.html")
        );

        // bad paths
        sanitize_path(base, "/../foo.html").expect_err("dot dot");

        sanitize_path(base, "/C:\\/foo.html").expect_err("C:\\");
    }
}
