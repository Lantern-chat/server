use futures::FutureExt;

use ftl::*;

use super::ApiError;

pub mod build;
pub mod file;
pub mod gateway;
pub mod invite;
pub mod party;
pub mod room;
pub mod user;
//pub mod metrics;

pub async fn api_v1(mut route: Route<crate::ServerState>) -> Response {
    match route.next().method_segment() {
        (_, Exact("user")) => user::user(route).boxed().await,
        (_, Exact("party")) => party::party(route).boxed().await,
        (_, Exact("file")) => file::file(route).boxed().await,
        (_, Exact("room")) => room::room(route).boxed().await,
        (_, Exact("gateway")) => gateway::gateway(route),
        (&Method::GET, Exact("build")) => build::build(route),
        //(&Method::GET, Exact("metrics")) => metrics::metrics(route),
        _ => ApiError::not_found().into_response(),
    }
}
