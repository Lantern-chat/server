use futures::FutureExt;

use ftl::Route;

pub use crate::ctrl::auth::Authorization;

use crate::ctrl::{
    auth::{self, AuthToken},
    Error,
};
use crate::ServerState;

pub async fn authorize(route: &Route<ServerState>) -> Result<Authorization, Error> {
    const BEARER: &[u8] = b"Bearer ";

    let header = match route.req.headers().get("Authorization") {
        Some(header) => header.as_bytes(),
        None => return Err(Error::MissingAuthorizationHeader),
    };

    if !header.starts_with(BEARER) {
        return Err(Error::InvalidAuthFormat);
    }

    auth::do_auth(&route.state, &header[BEARER.len()..]).await
}
