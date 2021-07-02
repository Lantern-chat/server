use ftl::*;

use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, user::me::friends::friends as list_friends},
    web::routes::api::ApiError,
    ServerState,
};

pub async fn friends(route: Route<ServerState>, auth: Authorization) -> Response {
    match list_friends(route.state, auth).await {
        Ok(sessions) => reply::json_stream(sessions).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
