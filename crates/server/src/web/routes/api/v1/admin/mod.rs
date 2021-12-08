use ftl::*;
use models::{ElevationLevel, UserFlags};
use schema::Snowflake;

use crate::web::{
    auth::{authorize, Authorization},
    routes::api::ApiError,
};

pub async fn admin(mut route: Route<crate::ServerState>) -> Response {
    let auth = match authorize(&route).await {
        Ok(auth) => match auth.flags.elevation() {
            ElevationLevel::Staff | ElevationLevel::System => auth,
            _ => return ApiError::not_found().into_response(),
        },
        _ => return ApiError::not_found().into_response(),
    };

    ().into_response()
}
