pub mod get;
pub mod redeem;
pub mod revoke;

use smol_str::SmolStr;

use super::*;

pub fn invite(mut route: Route<ServerState>, auth: MaybeAuth) -> RouteResult {
    let auth = auth.unwrap()?;

    match route.next().segment() {
        Exact(_) => match route.param::<SmolStr>() {
            Some(Ok(code)) => match route.next().method_segment() {
                (&Method::GET, End) => Ok(get::get_invite(route, auth, code)),
                (&Method::POST, Exact("redeem")) => Ok(redeem::redeem(route, auth, code)),
                (&Method::DELETE, Exact("revoke")) => Ok(revoke::revoke(route, auth, code)),

                _ => err(CommonError::NotFound),
            },
            Some(Err(_)) => err(CommonError::BadRequest),
            None => err(CommonError::NotFound),
        },
        _ => err(CommonError::NotFound),
    }
}
