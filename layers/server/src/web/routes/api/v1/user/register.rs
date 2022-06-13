use ftl::*;

use crate::{Error, ServerState};

pub async fn register(mut route: Route<ServerState>) -> Result<Response, Error> {
    let session = crate::backend::api::user::register::register_user(
        &route.state,
        route.real_addr,
        body::any(&mut route).await?,
    )
    .await?;

    Ok(reply::json(&session)
        .with_status(StatusCode::CREATED)
        .into_response())
}
