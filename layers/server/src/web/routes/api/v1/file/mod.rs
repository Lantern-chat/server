use http::{header::HeaderName, HeaderMap, HeaderValue, Method, StatusCode};

// https://tus.io/protocols/resumable-upload.html

lazy_static::lazy_static! {
    pub static ref TUS_HEADERS: HeaderMap<HeaderValue> = {
        let mut headers = HeaderMap::new();

        headers.insert(
            HeaderName::from_static("Tus-Resumable"),
            HeaderValue::from_static("1.0.0")
        );
        headers.insert(
            HeaderName::from_static("Tus-Version"),
            HeaderValue::from_static("1.0.0")
        );
        headers.insert(
            HeaderName::from_static("Tus-Extension"),
            HeaderValue::from_static("creation,expiration,checksum,termination")
        );
        headers.insert(
            HeaderName::from_static("Tus-Checksum-Algorithm"),
            HeaderValue::from_static("crc32")
        );

        headers
    };
}

fn tus_headers() -> impl Iterator<Item = (headers::HeaderName, HeaderValue)> {
    TUS_HEADERS.iter().map(|t| t.clone_tuple())
}

use super::*;
use crate::util::TupleClone;

pub mod head;
pub mod options;
pub mod patch;
pub mod post;

pub fn file(mut route: Route<ServerState>, auth: MaybeAuth) -> RouteResult {
    let auth = auth.unwrap()?;

    match route.next().method_segment() {
        (&Method::OPTIONS, End) => Ok(options::options(route, auth)),

        (&Method::POST, End) => Ok(post::post(route, auth)),

        (&Method::HEAD | &Method::PATCH | &Method::DELETE, Exact(_)) => {
            match route.param::<Snowflake>() {
                Some(Ok(file_id)) => {
                    // nothing should be after the file_id
                    if route.next().segment() != End {
                        return Err(Error::NotFound);
                    }

                    match route.method() {
                        &Method::HEAD => Ok(head::head(route, auth, file_id)),
                        &Method::PATCH => Ok(patch::patch(route, auth, file_id)),

                        _ => Err(Error::MethodNotAllowed),
                    }
                }
                _ => Err(Error::BadRequest),
            }
        }

        (_, Exact(_)) => Err(Error::MethodNotAllowed),

        _ => Err(Error::NotFound),
    }
}

fn header_from_int<T>(x: T) -> HeaderValue
where
    T: itoa::Integer,
{
    let mut buf = itoa::Buffer::new();
    match HeaderValue::from_str(buf.format(x)) {
        Ok(header) => header,
        Err(_) => unsafe { std::hint::unreachable_unchecked() },
    }
}
