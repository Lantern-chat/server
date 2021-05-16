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

//impl Reply for AuthError {
//    fn into_response(self) -> Response {
//        match self {
//            // TODO: Maybe don't include decode error?
//            AuthError::ClientError(_) | AuthError::DecodeError(_) => {
//                log::error!("Auth Error: {}", self);
//                StatusCode::INTERNAL_SERVER_ERROR.into_response()
//            }
//            _ => self
//                .to_string()
//                .with_status(StatusCode::BAD_REQUEST)
//                .into_response(),
//        }
//    }
//}
