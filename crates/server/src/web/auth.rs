use futures::FutureExt;

use ftl::Route;

use schema::auth::RawAuthToken;

pub use crate::ctrl::auth::Authorization;

use crate::ctrl::{auth, Error};
use crate::ServerState;

pub async fn authorize(route: &Route<ServerState>) -> Result<Authorization, Error> {
    let header = match route.req.headers().get("Authorization") {
        Some(header) => header.as_bytes(),
        None => return Err(Error::MissingAuthorizationHeader),
    };

    let token = RawAuthToken::try_from(header)?;

    auth::do_auth(&route.state, token).await
}
