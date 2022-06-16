use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::{Authorization, ServerState};

pub async fn friends(
    route: Route<ServerState>,
    auth: Authorization,
) -> ApiResponse {
    let friends =
        crate::backend::api::user::me::friends::friends(route.state, auth)
            .await?;

    Ok(reply::json::array_stream(friends).into_response())
}
