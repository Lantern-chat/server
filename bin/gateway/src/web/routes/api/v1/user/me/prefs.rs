use super::*;

#[async_recursion]
pub async fn prefs(mut route: Route<ServerState>, auth: Authorization) -> WebResult {
    let prefs = body::any(&mut route).await?;

    crate::backend::api::user::me::prefs::update_prefs(route.state, auth, prefs).await?;

    Ok(StatusCode::OK.into())
}
