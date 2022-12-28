use ftl::*;

use schema::Snowflake;

use super::WebResult;
use crate::{Authorization, ServerState};

#[async_recursion]
pub async fn get(route: Route<ServerState>, auth: Authorization) -> WebResult {
    let friends = crate::backend::api::user::me::friends::get::friends(route.state, auth).await?;

    Ok(reply::json::array_stream(friends).into_response())
}

#[async_recursion]
pub async fn post(route: Route<ServerState>, auth: Authorization, user_id: Snowflake) -> WebResult {
    crate::backend::api::user::me::friends::add::add_friend(route.state, auth, user_id).await?;

    Ok(().into_response())
}

#[async_recursion]
pub async fn del(route: Route<ServerState>, auth: Authorization, user_id: Snowflake) -> WebResult {
    crate::backend::api::user::me::friends::remove::remove_friend(route.state, auth, user_id).await?;

    Ok(().into_response())
}
