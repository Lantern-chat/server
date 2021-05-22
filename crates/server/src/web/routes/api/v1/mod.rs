use ftl::*;

use super::ApiError;

pub mod build;
//pub mod file;
pub mod gateway;
pub mod party;
//pub mod room;
pub mod user;

pub mod test;

pub async fn api_v1(mut route: Route<crate::ServerState>) -> impl Reply {
    match route.next().method_segment() {
        (_, Exact("user")) => user::user(route).await.into_response(),
        (_, Exact("party")) => party::party(route).await.into_response(),
        //(_, Exact("file")) => file::file(route).await.into_response(),
        //(_, Exact("room")) => room::room(route).await.into_response(),
        (_, Exact("gateway")) => gateway::gateway(route).into_response(),
        (&Method::GET, Exact("build")) => build::build().into_response(),

        (&Method::GET, Exact("test")) => test::test(route).await.into_response(),

        _ => ApiError::not_found().into_response(),
    }
}
