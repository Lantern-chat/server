use super::*;

use sdk::models::{ElevationLevel, UserFlags};

pub fn admin(mut route: Route<ServerState>, auth: MaybeAuth) -> RouteResult {
    match auth.0 {
        Some(Authorization::User { flags, .. })
            if matches!(flags.elevation(), ElevationLevel::Staff | ElevationLevel::System) => {}
        _ => return err(CommonError::NotFound),
    }

    err(CommonError::Unimplemented)
}
