use ftl::*;

use crate::{
    web::{auth::Authorization, routes::api::ApiError},
    ServerState,
};

use sdk::api::commands::invite::CreateInviteBody;

pub async fn post(mut route: Route<ServerState>, auth: Authorization) -> Response {
    let form = match body::any::<CreateInviteBody, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return ApiError::err(e.into()).into_response(),
    };

    ().into_response()
}
