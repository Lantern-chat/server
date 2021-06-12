use futures::FutureExt;

use ftl::*;

use headers::ContentType;

pub mod api;

pub async fn entry(mut route: Route<crate::ServerState>) -> Response {
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

        (&Method::GET, Exact("favicon.ico")) | (&Method::HEAD, Exact("favicon.ico")) => {
            fs::file(&route, "frontend/dist/favicon.ico", &route.state.file_cache)
                .boxed()
                .await
                .into_response()
        }

        _ if BAD_PATTERNS.is_match(route.path()) => StatusCode::IM_A_TEAPOT.into_response(),

        (&Method::GET, Exact("static")) | (&Method::HEAD, Exact("static")) => {
            fs::dir(&route, "frontend/dist", &route.state.file_cache)
                .boxed()
                .await
                .into_response()
        }

        (&Method::GET, segment) | (&Method::HEAD, segment) => {
            let allowed = match segment {
                Segment::End => true,
                Segment::Exact(part) => matches!(
                    part,
                    "channels" | "login" | "register" | "invite" // | "verify" | "profile" | "reset"
                ),
            };

            if !allowed {
                return StatusCode::NOT_FOUND.into_response();
            }

            fs::file(&route, "frontend/dist/index.html", &route.state.file_cache)
                .boxed()
                .await
                .into_response()
        }

        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

use aho_corasick::{AhoCorasick, AhoCorasickBuilder};

lazy_static::lazy_static! {
    static ref BAD_PATTERNS: AhoCorasick = AhoCorasickBuilder::new().dfa(true).build(&[
        "wp-includes", "wp-login", "wp-content", "wordpress", "xmlrpc.php",
        "wlwmanifest", ".git", "drupal", "ajax", "claro", "wp-json"
    ]);
}
