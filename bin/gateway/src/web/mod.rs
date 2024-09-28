use std::time::Duration;

use http::{HeaderName, HeaderValue, Method, StatusCode};

use ftl::{
    body::deferred::Deferred,
    extract::{MatchedPath, State},
    layers::rate_limit::{Error as RateLimitError, RateLimitLayerBuilder, RateLimitService},
    router::{HandlerService, Router},
    service::{Service, ServiceFuture},
    IntoResponse, Request, RequestParts, Response,
};

use crate::prelude::*;

pub mod build;
pub mod file_cache;
pub mod layers;
pub mod api {
    pub mod v1;
}

type InnerWebService = HandlerService<ServerState, Response>;

pub struct WebService {
    pub web: Router<ServerState, Response, RateLimitService<InnerWebService>>,
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

            match self.web.call_opt(req).await {
                Ok(Some(resp)) => Ok(resp),
                Ok(None) => Ok(StatusCode::NOT_FOUND.into_response()),
                Err(RateLimitError::RateLimit(err)) => Ok(err.into_response()),
            }
        }
    }
}

impl WebService {
    pub fn new(state: ServerState) -> Self {
        use ftl::layers::rate_limit::gcra::Quota;

        // Web routes are primarily used by actual humans, so configure it to be more strict
        // and disallow burst requests in the default quota.
        let mut rl = RateLimitLayerBuilder::new()
            .with_global_fallback(true)
            .with_default_quota(Duration::from_millis(5).into());

        let mut web = Router::with_state(state.clone());

        macro_rules! add_routes {
            ($($($method:ident)|+ $path:literal $(($emission_interval:expr $(; $burst:expr)?))? => $handler:expr),* $(,)?) => {
                $({
                    let methods = [$(Method::$method),+];

                    $(
                        let quota = Quota::new(
                            Duration::from_millis($emission_interval),
                            ($(core::num::NonZeroU64::new($burst).unwrap(),)? core::num::NonZeroU64::MIN,).0
                        );

                        for method in &methods {
                            rl.add_route((method.clone(), $path), quota);
                        }
                    )?

                    web.on(methods, $path, $handler);
                })*
            };
        }

        add_routes! {
            GET "/robots.txt" => || core::future::ready(include_str!("robots.txt")),
            GET "/build" (50) => build::build_info,
            GET|HEAD "/favicon.ico" => favicon,
            GET|HEAD "/static/{*path}" => static_files,
            GET|HEAD "/{*page}" => index_file,
        }

        // wildcard for GET/HEAD handled by index_file, so any others are simply disallowed
        web.fallback(|| async { StatusCode::METHOD_NOT_ALLOWED });

        Self {
            web: web.route_layer(rl.build()),
            api_v1: api::v1::ApiV1Service::new(state.clone()),
        }
    }
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
