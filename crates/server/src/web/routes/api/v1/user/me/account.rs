use ftl::*;

use models::Snowflake;

use crate::{
    ctrl::user::me::account::ModifyAccountForm,
    web::{auth::Authorization, routes::api::ApiError},
    ServerState,
};

pub async fn patch_account(mut route: Route<ServerState>, auth: Authorization) -> Response {
    let form = match body::any::<ModifyAccountForm, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return ApiError::err(e.into()).into_response(),
    };

    match crate::ctrl::user::me::account::modify_account(route.state, auth, form).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => ApiError::err(e).into_response(),
    }
}
