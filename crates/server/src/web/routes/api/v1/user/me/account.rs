use ftl::*;

use models::Snowflake;

use crate::{
    web::{auth::Authorization, routes::api::ApiError},
    ServerState,
};

pub async fn patch_account(mut route: Route<ServerState>, auth: Authorization) -> Response {
    ().into_response()
}
