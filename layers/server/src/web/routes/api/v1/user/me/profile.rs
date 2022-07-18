use ftl::*;

use super::ApiResponse;
use crate::{Authorization, Error, ServerState};

pub async fn patch_profile(mut route: Route<ServerState>, auth: Authorization) -> ApiResponse {
    let form = body::any(&mut route).await?;

    let profile =
        crate::backend::api::user::me::profile::patch_profile(route.state, auth, form, None).await?;

    Ok(reply::json(profile).into_response())
}
