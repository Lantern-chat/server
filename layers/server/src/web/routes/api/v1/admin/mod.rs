use super::*;

use sdk::models::{ElevationLevel, UserFlags};

pub fn admin(mut route: Route<ServerState>, auth: MaybeAuth) -> RouteResult {
    let auth = auth.unwrap().map_err(|_| Error::NotFound)?;

    if !matches!(auth.flags.elevation(), ElevationLevel::Staff | ElevationLevel::System) {
        return Err(Error::NotFound);
    }

    Err(Error::Unimplemented)
}
