use super::*;

#[async_recursion]
pub async fn get(route: Route<ServerState>, auth: Authorization, user_id: Snowflake) -> WebResult {
    Ok(WebResponse::new(
        crate::backend::api::user::get::get_full_user(route.state, auth, user_id).await?,
    ))
}
