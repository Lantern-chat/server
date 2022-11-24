use ftl::*;

use schema::Snowflake;

use super::ApiResponse;
use crate::{Authorization, ServerState};

#[async_recursion]
pub async fn get(
    route: Route<ServerState>,
    auth: Authorization,
    party_id: Snowflake,
) -> ApiResponse {
    let party =
        crate::backend::api::party::get::get_party(route.state, auth, party_id)
            .await?;

    Ok(reply::json(party).into_response())
}
