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
    let auth = crate::web::auth::maybe_authorize(&route).await?;

    let route_res = match route.next().method_segment() {
        (_, Exact("user")) => user::user(route, auth),
        (_, Exact("party")) => party::party(route, auth),
        (_, Exact("file")) => file::file(route, auth),
        (_, Exact("room")) => room::room(route, auth),
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
