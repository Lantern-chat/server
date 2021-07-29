use http::{HeaderMap, HeaderValue, Method, StatusCode};

use ftl::*;
use models::Snowflake;

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

use crate::{
    ctrl::Error,
    web::{auth::authorize, routes::api::ApiError},
    ServerState,
};

pub mod head;
pub mod options;
pub mod patch;
pub mod post;

pub async fn file(mut route: Route<ServerState>) -> Response {
    match route.next().method_segment() {
        // allow OPTIONS without authorization
        (&Method::OPTIONS, End) => options::options(route),

        _ => {
            let auth = match authorize(&route).await {
                Ok(auth) => auth,
                Err(e) => return ApiError::err(e).into_response(),
            };

            match route.method_segment() {
                (&Method::POST, End) => post::post(route, auth).await,

                (&Method::HEAD | &Method::PATCH | &Method::DELETE, Exact(_)) => {
                    match route.param::<Snowflake>() {
                        Some(Ok(file_id)) => {
                            // nothing should be after the file_id
                            if route.next().segment() != End {
                                return ApiError::not_found().into_response();
                            }

                            match *route.method() {
                                Method::HEAD => head::head(route, auth, file_id).await,
                                Method::PATCH => patch::patch(route, auth, file_id).await,

                                _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
                            }
                        }
                        _ => ApiError::bad_request().into_response(),
                    }
                }

                (_, Exact(_)) => StatusCode::METHOD_NOT_ALLOWED.into_response(),

                _ => ApiError::not_found().into_response(),
            }
        }
    }
}
