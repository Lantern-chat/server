use super::*;

#[async_recursion]
pub async fn post_2fa(mut route: Route<ServerState>, auth: Authorization) -> WebResult {
    let form = body::any(&mut route).await?;

    Ok(WebResponse::new(
        crate::backend::api::user::me::mfa::enable_2fa(route.state, auth.user_id(), form).await?,
    ))
}

#[async_recursion]
pub async fn patch_2fa(mut route: Route<ServerState>, auth: Authorization) -> WebResult {
    let form = body::any(&mut route).await?;

    Ok(WebResponse::new(
        crate::backend::api::user::me::mfa::confirm_2fa(route.state, auth.user_id(), form).await?,
    ))
}

#[async_recursion]
pub async fn delete_2fa(mut route: Route<ServerState>, auth: Authorization) -> WebResult {
    let form = body::any(&mut route).await?;

    Ok(WebResponse::new(
        crate::backend::api::user::me::mfa::remove_2fa(route.state, auth.user_id(), form).await?,
    ))
}
