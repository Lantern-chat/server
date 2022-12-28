use super::*;

#[async_recursion]
pub async fn patch_account(mut route: Route<ServerState>, auth: Authorization) -> WebResult {
    let form = body::any(&mut route).await?;

    crate::backend::api::user::me::account::modify_account(route.state, auth, form).await?;

    Ok(StatusCode::OK.into())
}
