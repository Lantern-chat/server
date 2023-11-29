use super::*;

#[async_recursion]
pub async fn register(mut route: Route<ServerState>) -> WebResult {
    let form = body::any(&mut route).await?;

    Ok(WebResponse::new(
        crate::backend::api::user::register::register_user(route.state, route.real_addr, form).await?,
    )
    .with_status(StatusCode::CREATED))
}
