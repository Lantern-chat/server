use super::*;

#[async_recursion]
pub async fn post(mut route: Route<ServerState>, auth: Authorization) -> WebResult {
    let form = body::any(&mut route).await?;

    Ok(WebResponse::new(
        crate::backend::api::party::create::create_party(route.state, auth, form).await?,
    ))
}
