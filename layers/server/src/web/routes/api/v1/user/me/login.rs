use ftl::*;

use crate::{Error, ServerState};

pub async fn login(mut route: Route<ServerState>) -> Result<Response, Error> {
    let session = crate::backend::api::user::me::login::login(
        &route.state,
        route.real_addr,
        body::any(&mut route).await?,
    )
    .await?;

    Ok(reply::json(&session)
        .with_status(StatusCode::CREATED)
        .into_response())
}
