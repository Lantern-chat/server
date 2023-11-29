use headers::HeaderName;

use ftl::Route;

use schema::auth::RawAuthToken;

use crate::backend::api::auth;
pub use crate::backend::Authorization;

use crate::{Error, ServerState};

const AUTH_HEADER: &str = "authorization";

pub async fn authorize(route: &Route<ServerState>) -> Result<Authorization, Error> {
    let header = match route.raw_header(HeaderName::from_static(AUTH_HEADER)) {
        Some(header) => header.to_str()?,
        None => return Err(Error::MissingAuthorizationHeader),
    };

    let auth = auth::do_auth(&route.state, RawAuthToken::from_header(header)?).await?;

    Ok(auth)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct MaybeAuth(pub Option<Authorization>);

impl MaybeAuth {
    pub fn unwrap(self) -> Result<Authorization, Error> {
        match self.0 {
            Some(auth) => Ok(auth),
            None => Err(Error::Unauthorized),
        }
    }
}

pub async fn maybe_authorize(route: &Route<ServerState>) -> Result<MaybeAuth, Error> {
    match route.raw_header(HeaderName::from_static(AUTH_HEADER)) {
        None => Ok(MaybeAuth(None)),
        Some(header) => auth::do_auth(&route.state, RawAuthToken::from_header(header.to_str()?)?)
            .await
            .map(|auth| MaybeAuth(Some(auth))),
    }
}
