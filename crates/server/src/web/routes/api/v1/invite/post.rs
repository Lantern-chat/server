use ftl::*;

use crate::{
    web::{auth::Authorization, routes::api::ApiError},
    ServerState,
};

#[derive(Debug, Deserialize)]
pub struct PostInviteForm {
    #[serde(default)]
    max_uses: Option<u16>,

    #[serde(default)]
    expires: Option<String>,
}

pub async fn post(mut route: Route<ServerState>, auth: Authorization) -> Response {
    let form = match body::any::<PostInviteForm, _>(&mut route).await {
        Ok(form) => form,
        Err(e) => return ApiError::err(e.into()).into_response(),
    };

    ().into_response()
}
