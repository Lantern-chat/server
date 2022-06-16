use http::{HeaderMap, HeaderValue, Method, StatusCode};

use ftl::*;
use sdk::models::Snowflake;

// https://tus.io/protocols/resumable-upload.html

lazy_static::lazy_static! {
    pub static ref TUS_HEADERS: HeaderMap<HeaderValue> = {
        let mut headers = HeaderMap::new();

        headers.insert("Tus-Resumable", HeaderValue::from_static("1.0.0"));
        headers.insert("Tus-Version", HeaderValue::from_static("1.0.0"));
        headers.insert("Tus-Extension", HeaderValue::from_static("creation,expiration,checksum,termination"));
        headers.insert("Tus-Checksum-Algorithm", HeaderValue::from_static("crc32"));

        headers
    };
}

fn tus_headers() -> impl Iterator<Item = (headers::HeaderName, HeaderValue)> {
    TUS_HEADERS.iter().map(|t| t.clone_tuple())
}

use super::ApiResponse;
use crate::{util::TupleClone, Error, ServerState};

pub mod head;
pub mod options;
pub mod patch;
pub mod post;

pub async fn file(mut route: Route<ServerState>) -> ApiResponse {
    let auth = crate::web::auth::authorize(&route).await?;

    match route.next().method_segment() {
        (&Method::OPTIONS, End) => options::options(route, auth).await,

        (&Method::POST, End) => post::post(route, auth).await,

        (&Method::HEAD | &Method::PATCH | &Method::DELETE, Exact(_)) => {
            match route.param::<Snowflake>() {
                Some(Ok(file_id)) => {
                    // nothing should be after the file_id
                    if route.next().segment() != End {
                        return Err(Error::NotFound);
                    }

                    match route.method() {
                        &Method::HEAD => head::head(route, auth, file_id).await,
                        &Method::PATCH => patch::patch(route, auth, file_id).await,

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
