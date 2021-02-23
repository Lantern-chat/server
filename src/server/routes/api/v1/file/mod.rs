use http::{HeaderMap, HeaderValue, Method, StatusCode};

use crate::{
    db::{schema::file::File, Snowflake},
    server::{ftl::*, ServerState},
};

pub mod delete;
pub mod head;
pub mod options;
pub mod patch;
pub mod post;

// https://tus.io/protocols/resumable-upload.html

lazy_static::lazy_static! {
    pub static ref TUS_HEADERS: HeaderMap<HeaderValue> = {
        let mut headers = HeaderMap::new();

        //let max_size = 1024 * 1024 * 100; // 100MB
        //let max_size_str = max_size.to_string();

        headers.insert("Tus-Resumable", HeaderValue::from_static("1.0.0"));
        headers.insert("Tus-Version", HeaderValue::from_static("1.0.0"));
        headers.insert("Tus-Extension", HeaderValue::from_static("creation,expiration,checksum,termination"));
        headers.insert("Tus-Checksum-Algorithm", HeaderValue::from_static("crc32"));
        //headers.insert("Tus-Max-Size", HeaderValue::from_str(&max_size_str).unwrap());

        headers
    };

    pub static ref CHECKSUM_MISMATCH: StatusCode = StatusCode::from_u16(460).unwrap();
}

pub async fn file(mut route: Route) -> impl Reply {
    match route.next().method_segment() {
        // POST /api/v1/file
        (&Method::POST, End) => post::post(route).await.into_response(),

        // OPTIONS /api/v1/file
        (&Method::OPTIONS, End) => options::options(route).await.into_response(),

        // ANY /api/v1/file/1234
        (_, Exact(_)) => match route.param::<Snowflake>() {
            Some(Ok(file_id)) => {
                if let Exact(_) = route.next().segment() {
                    // Nothing should be after the file_id
                    return StatusCode::NOT_FOUND.into_response();
                }

                // load file info from database
                let file = match File::find(file_id, &route.state.db).await {
                    Ok(Some(file)) => file,
                    Ok(None) => return StatusCode::NOT_FOUND.into_response(),
                    Err(e) => {
                        log::error!("Error getting file entry: {}", e);
                        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                    }
                };

                match route.method() {
                    // HEAD /api/v1/file/1234
                    &Method::HEAD => head::head(route, file).await.into_response(),

                    // PATCH /api/v1/file/1234
                    &Method::PATCH => patch::patch(route, file).await.into_response(),

                    // DELETE /api/v1/file/1234
                    &Method::DELETE => delete::delete(route, file).await.into_response(),

                    _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
                }
            }
            _ => StatusCode::BAD_REQUEST.into_response(),
        },

        _ => StatusCode::NOT_FOUND.into_response(),
    }
}
