use ftl::*;

use db::Snowflake;

use crate::{
    ctrl::{auth::Authorization, user::sessions::list_sessions},
    web::routes::api::ApiError,
    ServerState,
};

pub async fn sessions(route: Route<ServerState>, auth: Authorization) -> impl Reply {
    match list_sessions(route.state, auth).await {
        Ok(sessions) => reply::json_stream(sessions).into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
