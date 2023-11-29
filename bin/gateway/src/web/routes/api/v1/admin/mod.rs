use super::*;

use sdk::models::{ElevationLevel, UserFlags};

pub fn admin(mut route: Route<ServerState>, auth: MaybeAuth) -> RouteResult {
    match auth.0 {
        Some(Authorization::User {
            token,
            user_id,
            expires,
            flags,
        }) if matches!(flags.elevation(), ElevationLevel::Staff | ElevationLevel::System) => {}
        _ => return Err(Error::NotFound),
    }

    Err(Error::Unimplemented)
}
