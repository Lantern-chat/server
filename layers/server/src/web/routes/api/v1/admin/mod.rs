use ftl::*;
use schema::Snowflake;
use sdk::models::{ElevationLevel, UserFlags};

use crate::web::auth::{authorize, Authorization};
use crate::Error;

pub async fn admin(mut route: Route<crate::ServerState>) -> Result<Response, Error> {
    let auth = match authorize(&route).await {
        Ok(auth) => match auth.flags.elevation() {
            ElevationLevel::Staff | ElevationLevel::System => auth,
            _ => return Err(Error::NotFound),
        },
        _ => return Err(Error::NotFound),
    };

    Ok(().into_response())
}
