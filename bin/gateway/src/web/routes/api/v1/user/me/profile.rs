use super::*;

#[async_recursion]
pub async fn patch_profile(mut route: Route<ServerState>, auth: Authorization) -> WebResult {
    let form = body::any(&mut route).await?;

    Ok(WebResponse::new(
        crate::backend::api::user::me::profile::patch_profile(route.state, auth, form, None).await?,
    ))
}
