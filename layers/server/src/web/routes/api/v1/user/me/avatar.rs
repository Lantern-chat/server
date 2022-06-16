use ftl::*;

use sdk::models::Snowflake;

use super::ApiResponse;
use crate::{Authorization, Error, ServerState};

pub async fn post_avatar(
    mut route: Route<ServerState>,
    auth: Authorization,
) -> ApiResponse {
    let file_id = match route.next().param::<Snowflake>() {
        Some(Ok(file_id)) => file_id,
        _ => return Err(Error::BadRequest),
    };

    crate::backend::api::user::me::avatar::process_avatar(
        route.state,
        auth.user_id,
        file_id,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT.into_response())
}
