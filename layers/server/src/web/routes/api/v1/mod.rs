pub mod admin;
pub mod build;
pub mod file;
pub mod gateway;
pub mod invite;
pub mod metrics;
pub mod oembed;
pub mod party;
pub mod room;
pub mod user;

#[cfg(debug_assertions)]
pub mod debug;

use super::*;

pub async fn api_v1(mut route: Route<ServerState>) -> WebResult {
    route.next();

    // only `PATCH api/v1/file` is allowed to exceed this value
    if !(*route.method() == Method::PATCH && route.segment() == Exact("file")) {
        use hyper::body::HttpBody;

        // API Requests are limited to a body size of 1MiB
        // TODO: Reduce this eventually?
        if !matches!(route.body().size_hint().upper(), Some(len) if len <= (1024 * 1024)) {
            return Err(Error::RequestEntityTooLarge);
        }
    }

    let auth = crate::web::auth::maybe_authorize(&route).await?;

    let route_res = match route.method_segment() {
        (_, Exact("room")) => room::room(route, auth),
        (_, Exact("user")) => user::user(route, auth),
        (_, Exact("party")) => party::party(route, auth),
        (_, Exact("file")) => file::file(route, auth),
        (_, Exact("invite")) => invite::invite(route, auth),
        (_, Exact("gateway")) => return gateway::gateway(route),
        (&Method::GET, Exact("build")) => return build::build(route),
        (&Method::GET, Exact("metrics")) => Ok(metrics::metrics(route)),
        (&Method::GET, Exact("oembed")) => Ok(oembed::oembed(route)),
        (_, Exact("admin")) => admin::admin(route, auth),

        #[cfg(debug_assertions)]
        (_, Exact("debug")) => debug::debug(route),

        _ => Err(Error::NotFound),
    };

    match route_res {
        Ok(fut) => fut.await,
        Err(e) => Err(e),
    }
}
