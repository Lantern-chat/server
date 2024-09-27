use http::{HeaderName, HeaderValue, Method, StatusCode};

use ftl::{
    body::deferred::Deferred,
    extract::{MatchedPath, State},
    service::{Service, ServiceFuture},
    IntoResponse, Request, RequestParts, Response, Router,
};

use crate::prelude::*;

pub mod build;
pub mod file_cache;
pub mod layers;
pub mod api {
    pub mod v1;
}

pub struct WebService {
    pub web: Router<ServerState, Response>,
    pub api_v1: api::v1::ApiV1Service,
}

impl Service<Request> for WebService {
    type Error = Error;
    type Response = Response;

    fn call(&self, req: Request) -> impl ServiceFuture<Self::Response, Self::Error> {
        async move {
            let path = req.uri().path();

            if is_bad_pattern(path) {
                return Ok(StatusCode::IM_A_TEAPOT.into_response());
            }

            if path.starts_with("/api/v1/") {
                return self.api_v1.call(req).await;
            }

            match self.web.call(req).await {
                Ok(resp) => Ok(resp),
                Err(e) => Ok(e.into_response()),
            }
        }
    }
}

impl WebService {
    pub fn new(state: ServerState) -> Self {
        let mut web = Router::with_state(state.clone());

        web.get("/robots.txt", robots);
        web.get("/build", build::build_info);

        web.on([Method::GET, Method::HEAD], "/favicon.ico", favicon);
        web.on([Method::GET, Method::HEAD], "/static/{*path}", static_files);
        web.on([Method::GET, Method::HEAD], "/{*page}", index_file);

        // wildcard for GET/HEAD handled by index_file, so any others are simply disallowed
        web.fallback(|| async { StatusCode::METHOD_NOT_ALLOWED });

        Self {
            web,
            api_v1: api::v1::ApiV1Service::new(state.clone()),
        }
    }
}

async fn robots() -> &'static str {
    include_str!("robots.txt")
}

async fn static_files(State(state): State<ServerState>, path: MatchedPath, parts: RequestParts) -> Response {
    let base_dir = state.config().local.paths.web_path.join("dist");

    ftl::fs::dir(&parts, &state, &*path, base_dir, &state.file_cache).await
}

async fn index_file(State(state): State<ServerState>, parts: RequestParts) -> Response {
    // either empty path or one of the allowed paths
    #[rustfmt::skip]
    let allowed = matches!(parts.uri.path().split_once('/').map(|x| x.1),
        None | Some("" | "rooms" | "login" | "register" | "invite" | "verify" | "settings" | "reset")
    );

    // NOTE: Whitelisting paths deters a bunch of false requests from bots
    if !allowed {
        return StatusCode::NOT_FOUND.into_response();
    }

    let path = state.config().local.paths.web_path.join("dist/index.html");
    let mut resp = ftl::fs::file(&parts, &state, path, &state.file_cache).await;

    // TODO: Revisit this conclusion?
    // index.html is small, always fetch latest version
    resp.headers_mut().insert(
        const { HeaderName::from_static("cache-control") },
        const { HeaderValue::from_static("no-cache, no-store, must-revalidate, proxy-revalidate, max-age=0") },
    );

    // if let Some(hvalue) = gen_oembed_header_value(&route) {
    //     resp.headers_mut().insert(const { HeaderName::from_static("link") }, hvalue);
    // }

    resp
}

async fn favicon(State(state): State<ServerState>, parts: RequestParts) -> Response {
    let path = state.config().local.paths.web_path.join("assets/favicon.ico");
    ftl::fs::file(&parts, &state, path, &state.file_cache).await
}

fn is_bad_pattern(path: &str) -> bool {
    use aho_corasick::{AhoCorasick, AhoCorasickBuilder};

    use std::sync::LazyLock;

    #[rustfmt::skip]
    static BAD_PATTERNS: LazyLock<AhoCorasick> = LazyLock::new(|| {
        AhoCorasickBuilder::new().ascii_case_insensitive(true).build([
            "wp-includes", "wp-admin", "wp-login", "wp-content", "wordpress",
            "wlwmanifest", ".git", ".env", "drupal", "ajax", "claro", "wp-json", "tinymce", "kcfinder",
            "filemanager", "alfa", "eval"
        ]).unwrap()
    });

    path.ends_with(".php") || BAD_PATTERNS.is_match(path)
}

// fn gen_oembed_header_value(route: &Route<ServerState>) -> Option<HeaderValue> {
//     let host = route.host()?;

//     let path = format!("https://{}/{}", host.as_str(), route.path());

//     let value = format!(
//         r#"<https://lantern.chat/api/v1/oembed?format=json&url={}>; rel="alternate"; type="application/json+oembed""#,
//         urlencoding::encode(&path)
//     );

//     HeaderValue::from_str(&value).ok()
// }
