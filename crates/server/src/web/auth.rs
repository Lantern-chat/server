use ftl::Route;

use crate::ctrl::{
    auth::{self, AuthToken, Authorization},
    Error,
};
use crate::ServerState;

pub async fn authorize(route: &Route<ServerState>) -> Result<Authorization, Error> {
    const BEARER: &'static [u8] = b"Bearer ";

    let header = match route.req.headers().get("Authorization") {
        Some(header) => header.as_bytes(),
        None => return Err(Error::MissingAuthorizationHeader),
    };

    if !header.starts_with(BEARER) {
        return Err(Error::InvalidAuthFormat);
    }

    auth::do_auth(&route.state, &header[BEARER.len()..]).await
}
