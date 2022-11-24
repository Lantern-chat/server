pub mod get;
pub mod redeem;
pub mod revoke;

use ftl::*;
use futures::FutureExt;
use smol_str::SmolStr;

use super::ApiResponse;
use crate::{Error, ServerState};

pub async fn invite(mut route: Route<ServerState>) -> ApiResponse {
    let auth = crate::web::auth::authorize(&route).await?;

    match route.next().segment() {
        Exact(_) => match route.param::<SmolStr>() {
            Some(Ok(code)) => match route.next().method_segment() {
                (&Method::GET, End) => get::get_invite(route, auth, code).boxed().await,
                (&Method::POST, Exact("redeem")) => redeem::redeem(route, auth, code).boxed().await,
                (&Method::DELETE, Exact("revoke")) => revoke::revoke(route, auth, code).boxed().await,

                _ => Err(Error::NotFound),
            },
            Some(Err(_)) => Err(Error::BadRequest),
            None => Err(Error::NotFound),
        },
        _ => Err(Error::NotFound),
    }
}
