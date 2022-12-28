use super::*;

#[async_recursion]
pub async fn login(mut route: Route<ServerState>) -> WebResult {
    let form = body::any(&mut route).await?;

    Ok(WebResponse::new(
        crate::backend::api::user::me::login::login(route.state, route.real_addr, form).await?,
    )
    .with_status(StatusCode::CREATED))
}
