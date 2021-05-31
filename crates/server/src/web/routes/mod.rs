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
        (_, Exact("api")) => api::api(route).await,

        (_, Exact("robots.txt")) => include_str!("robots.txt")
            .with_header(ContentType::text())
            .into_response(),

        _ if BAD_PATTERNS.is_match(route.path()) => StatusCode::IM_A_TEAPOT.into_response(),

        (&Method::GET, Exact("static")) | (&Method::HEAD, Exact("static")) => {
            fs::dir(&route, "frontend/dist").await.into_response()
        }

        (&Method::GET, _) | (&Method::HEAD, _) => fs::file(&route, "frontend/dist/index.html")
            .await
            .into_response(),

        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

use aho_corasick::AhoCorasick;

lazy_static::lazy_static! {
    static ref BAD_PATTERNS: AhoCorasick = AhoCorasick::new(&[
        "wp-includes", "wp-login", "wp-content", "wordpress", "xmlrpc.php", "wlwmanifest"
    ]);
}
