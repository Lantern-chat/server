use futures::FutureExt;

use ftl::Route;

use schema::auth::RawAuthToken;

use crate::backend::api::auth;
pub use crate::backend::Authorization;

use crate::{Error, ServerState};

pub async fn authorize(route: &Route<ServerState>) -> Result<Authorization, Error> {
    let header = match route.req.headers().get("Authorization") {
        Some(header) => header.to_str()?,
        None => return Err(Error::MissingAuthorizationHeader),
    };

    let auth = auth::do_auth(&route.state, RawAuthToken::from_header(header)?).await?;

    Ok(auth)
}
