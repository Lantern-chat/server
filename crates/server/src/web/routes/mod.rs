use futures::FutureExt;

use ftl::*;

use headers::{ContentType, HeaderValue};

use crate::{web::routes::api::ApiError, ServerState};

pub mod api;
pub mod cdn;

pub async fn entry(mut route: Route<ServerState>) -> Response {
    if route.path().len() > 255 || route.raw_query().map(|q| q.len() > 255) == Some(true) {
        return ApiError::bad_request().into_response();
    }

    if let Err(_) = route.apply_method_override() {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }

    route.next();

    match route.method_segment() {
        // ANY /api
        (_, Exact("api")) => compression::wrap_route(true, route, api::api).await,

        (_, Exact("robots.txt")) => include_str!("robots.txt")
            .with_header(ContentType::text())
            .into_response(),

        (&Method::GET | &Method::HEAD, Exact("favicon.ico")) => {
            fs::file(&route, "frontend/assets/favicon.ico", &route.state.file_cache)
                .boxed()
                .await
                .into_response()
        }

        _ if BAD_PATTERNS.is_match(route.path()) => StatusCode::IM_A_TEAPOT.into_response(),

        (&Method::GET | &Method::HEAD, Exact("static")) => {
            fs::dir(&route, "frontend/dist", &route.state.file_cache)
                .boxed()
                .await
                .into_response()
        }

        (&Method::GET | &Method::HEAD, Exact("cdn")) => cdn::cdn(route).boxed().await,

        (&Method::GET | &Method::HEAD, segment) => {
            let allowed = match segment {
                Segment::End => true,
                Segment::Exact(part) => matches!(
                    part,
                    "channels" | "login" | "register" | "invite" | "verify" | "settings" | "reset"
                ),
            };

            if !allowed {
                return StatusCode::NOT_FOUND.into_response();
            }

            let mut resp = fs::file(&route, "frontend/dist/index.html", &route.state.file_cache)
                .boxed()
                .await
                .into_response();

            let headers = resp.headers_mut();

            if cfg!(debug_assertions) {
                headers.insert(
                    "Cache-Control",
                    HeaderValue::from_static(
                        "no-store, no-cache, must-revalidate, proxy-revalidate, max-age=0",
                    ),
                );
            }

            if let Some(hvalue) = gen_oembed_header_value(&route) {
                headers.insert("Link", hvalue);
            }

            resp
        }

        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

use aho_corasick::{AhoCorasick, AhoCorasickBuilder};

lazy_static::lazy_static! {
    static ref BAD_PATTERNS: AhoCorasick = AhoCorasickBuilder::new().dfa(true).build(&[
        "wp-includes", "wp-admin", "wp-login", "wp-content", "wordpress", "xmlrpc.php",
        "wlwmanifest", ".git", "drupal", "ajax", "claro", "wp-json"
    ]);
}

fn gen_oembed_header_value(route: &Route<ServerState>) -> Option<HeaderValue> {
    let host = route.host()?;

    let path = format!("https://{}/{}", host.as_str(), route.path());

    let value = format!(
        r#"<https://lantern.chat/api/v1/oembed?format=json&url={}>; rel="alternate"; type="application/json+oembed""#,
        urlencoding::encode(&path)
    );

    HeaderValue::from_str(&value).ok()
}
